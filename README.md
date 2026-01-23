# mp4-bmff-rs

ISO/IEC 14496-12 (ISO Base Media File Format, ISOBMFF) の純粋なRust実装です。

## 設計方針

### レイヤー構造

本クレートは、機能を段階的に提供する4層のレイヤー構造で設計されています。
各レイヤーの境界はFeature依存（`no_std`/`alloc`/`std`）で分けられています。

```
Layer 3: I/O (std)           - BoxReader / BoxWriter
Layer 2: Typed Boxes (alloc) - 型付きボックス表現
Layer 1: Core (no_std)       - ISO BMFF基本構造
Layer 0: Primitives          - FourCC, Fixed-point numbers, Matrix等
```

| レイヤー | 必要なFeature | 内容                                                         |
| -------- | ------------- | ------------------------------------------------------------ |
| Layer 0  | なし          | プリミティブ型（`FourCC`, `Fixed<I,F>`, `Matrix`, `Uuid`等） |
| Layer 1  | なし          | ISO BMFF基本構造（詳細は下表）                               |
| Layer 2  | `alloc`       | 型付きボックス表現（`FtypBox`, `MoovBox`等）                 |
| Layer 3  | `std`         | ストリームベースのI/O（`BoxReader`, `BoxWriter`）            |

**Layer 1 の構成:**

| モジュール | 責務                                                                |
| ---------- | ------------------------------------------------------------------- |
| `base`     | データ構造（`BoxHeader`, `BoxType`, `RawBox`等）                    |
| `error`    | エラー型（`Error`, `ErrorKind`）                                    |
| `codec`    | エンコード/デコードトレイト（`BoxCodec`, `BoxDecode`, `BoxEncode`） |
| `iter`     | イテレータ（`BoxIter`, `FixedSizeEntryIter`）                       |

`codec`と`iter`は、Layer 1のデータ構造とLayer 2の型付きボックスを繋ぐトレイト・ヘルパーを提供します。

### View/Owned パターン

可変長データを持つボックスには、2種類の型を提供しています。

- **View型** (`XxxBoxView<'a>`): バイトスライスへの参照を保持し、ゼロコピーパースを実現
- **Owned型** (`XxxBox`): データを所有し、変更や書き込みが可能

```rust
// View型: ゼロコピーでパース（no_std/no_alloc環境でも使用可能）
let ftyp_view: FtypBoxView = FtypBoxView::decode(payload)?;
println!("Major brand: {}", ftyp_view.major_brand);

// Owned型: データを所有（alloc feature必要）
let ftyp: FtypBox = FtypBox::from(&ftyp_view);
ftyp.compatible_brands.push(FourCC::new(*b"iso6"));
```

### Copy型ボックス

固定サイズのボックス（`MvhdBox`, `TkhdBox`等）は、View/Owned の区別がなく単一の`Copy`可能な型として実装されています。

```rust
let mvhd: MvhdBox = MvhdBox::decode(payload)?;
let copy = mvhd; // Copy可能
```

### トレイトベース設計

ボックスのエンコード/デコードは、以下のトレイトで抽象化されています。

| トレイト         | 役割                                 |
| ---------------- | ------------------------------------ |
| `BoxCodec`       | ボックスタイプ（FourCC）を返す       |
| `BoxDecode<'de>` | バイトスライスからボックスをデコード |
| `BoxEncode`      | ボックスをバイトスライスにエンコード |

```rust
pub trait BoxCodec {
    fn boxtype(&self) -> BoxType;
}

pub trait BoxDecode<'de>: Sized {
    fn decode(bytes: &'de [u8]) -> Result<Self>;
}

pub trait BoxEncode {
    fn encoded_len(&self) -> usize;
    fn encode(&self, bytes: &mut [u8]) -> Result<()>;
}
```

### コンテナボックス

コンテナボックス（`moov`, `trak`等）は、子ボックスへのアクセスにイテレータを使用します。

```rust
let moov: MoovBoxView = MoovBoxView::decode(payload)?;

// 必須ボックスは専用メソッドでアクセス
let mvhd: MvhdBox = moov.mvhd()?;

// 複数存在する子ボックスはイテレータでアクセス
for trak in moov.traks() {
    let trak = trak?;
    let tkhd = trak.tkhd()?;
    println!("Track ID: {}", tkhd.track_id);
}
```

## Feature Flags

| Feature | デフォルト | 説明                                  |
| ------- | ---------- | ------------------------------------- |
| `std`   | 有効       | 標準ライブラリサポート。`alloc`を含む |
| `alloc` | -          | ヒープ割り当て。Owned型ボックスに必要 |

```toml
# デフォルト（std有効）
mp4-bmff = "0.1"

# no_std + alloc
mp4-bmff = { version = "0.1", default-features = false, features = ["alloc"] }

# no_std + no_alloc（View型のみ使用可能）
mp4-bmff = { version = "0.1", default-features = false }
```

## 使用例

### ファイルからボックスを読み込む

```rust
use std::fs::File;
use mp4_bmff::{BoxReader, BoxType};
use mp4_bmff::boxes::MoovBoxView;

let file = File::open("video.mp4")?;
let reader = BoxReader::new(file);

for raw in reader {
    let raw = raw?;
    match raw.boxtype() {
        BoxType::MOOV => {
            let moov = MoovBoxView::decode(raw.payload())?;
            let mvhd = moov.mvhd()?;
            println!("Duration: {} (timescale: {})", mvhd.duration, mvhd.timescale);
        }
        _ => {}
    }
}
```

### ボックスを書き込む

```rust
use mp4_bmff::{BoxWriter, BoxType};
use mp4_bmff::boxes::FtypBox;
use mp4_bmff::types::FourCC;

let ftyp = FtypBox {
    major_brand: FourCC::new(*b"isom"),
    minor_version: 512,
    compatible_brands: vec![
        FourCC::new(*b"isom"),
        FourCC::new(*b"iso2"),
        FourCC::new(*b"avc1"),
    ],
};

let mut output = Vec::new();
let mut writer = BoxWriter::new(&mut output);
writer.write_box(&ftyp)?;
```

## 対応ボックス

主要なISO BMFF / MP4ボックスに対応しています。詳細は `src/boxes.rs` のボックス一覧を参照してください。

## ライセンス

MIT OR Apache-2.0
