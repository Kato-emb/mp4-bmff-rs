# fmp4-asm Web demo

ブラウザから fMP4 セグメントのディレクトリを選択して、1 つの非フラグメント MP4
に結合し、結果をダウンロードする最小デモです。

`examples/src/fmp4_asm.rs` (CLI) と同じロジックを WASM 経由で実行します。

## 構成

```
web/
├── index.html   ← UI (main thread)
├── main.js      ← ディレクトリ選択 + Worker 起動
├── worker.js    ← WASM ロード + OPFS 書込み (Dedicated Worker)
├── build.sh     ← cargo + wasm-bindgen を走らせるヘルパ
└── pkg/         ← (build.sh が生成) wasm-bindgen 出力
```

## ビルド

事前に以下を一度だけ:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli   # Cargo.lock の wasm-bindgen と揃えること
```

ビルド:

```bash
cd wasm/fmp4-asm/web
./build.sh           # debug
./build.sh release   # release (推奨: wasm が小さい)
```

`./pkg/mp4_bmff_wasm.js` と `./pkg/mp4_bmff_wasm_bg.wasm` が出力されます。

## 配信

OPFS の `FileSystemSyncAccessHandle` API は **Worker 専用 + Secure Context**
が必要ですが、ブラウザは `localhost` を Secure Context として扱うので
HTTPS は不要です。以下のいずれかで配信してください。

### A. ローカル開発

```bash
cd wasm/fmp4-asm/web
python3 -m http.server 8080
# ブラウザで http://localhost:8080 を開く
```

### B. SSH 越しのリモート開発

リモート (コンテナ内) で HTTP を起動し、SSH ローカルポートフォワードで
手元のブラウザから `localhost` として見える状態にすれば Secure Context
扱いになります。

リモート:
```bash
python3 -m http.server 8080
```

ローカル:
```bash
ssh -L 8080:localhost:8080 user@remote-host
# VS Code Remote-SSH なら Ports タブで 8080 を Forward すれば自動
```

ローカルブラウザで `http://localhost:8080` を開く。

## 動作要件

- **Chromium 系ブラウザ** (Chrome, Edge) を推奨。Firefox は OPFS
  SyncAccessHandle のサポート状況がバージョン依存。
- ディレクトリピッカは `<input type="file" webkitdirectory>` を使用。

## 使い方

1. 「ディレクトリ選択」でフラグメントが入ったディレクトリを選ぶ。
   - `init.mp4` / `init.cmfi` / `init.m4i` が 1 つ
   - `*.m4s` / `*.m4v` / `*.m4a` / `*.cmfv` / `*.cmfa` / `*.cmft` が 1 つ以上
2. 「結合を実行」をクリック。
3. 完了すると「結果」セクションにダウンロードリンクが現れる。

## 動作の流れ

```
[main thread]
  └─ FileList を読み込み (init を分離、fragment をソート)
       │ postMessage(init bytes + fragment bytes 配列)
       ▼
[Worker]
  ├─ wasm-bindgen init()
  ├─ navigator.storage.getDirectory() → OPFS root
  ├─ getFileHandle(tmp, { create: true })
  ├─ createSyncAccessHandle()
  ├─ wasm.initialize(syncHandle)
  ├─ wasm.process_init_segment(initBytes)
  ├─ for each fragment: wasm.process_media_segment(bytes)
  ├─ wasm.finalize(new Date())
  ├─ syncHandle.close()
  ├─ getFile() → Blob
  └─ removeEntry(tmp)  // OPFS のテンポラリは掃除
       │ postMessage(blob)
       ▼
[main thread]
  └─ URL.createObjectURL(blob) → <a download> でユーザーに提示
```

## トラブルシュート

- **`createSyncAccessHandle is not a function`**: ブラウザが対応していない、
  または Worker ではなく main thread から呼んでいる。
- **`The user aborted a request.`**: 既に他のタブが同じ OPFS ファイルに
  アクセスしている。テンポラリ名を Date.now() で生成しているのでまず
  発生しないはず。
- **ダウンロードファイルが壊れている**: 入力 fragment が CMAF 非準拠
  (例: tfhd の `default_base_is_moof` フラグなしで base_data_offset も
  なし) の場合エラーになります。
