# @kato-emb/mp4-bmff-wasm

Stream fragmented MP4 (fMP4 / CMAF) segments into a single non-fragmented MP4
file from the browser, using OPFS as scratch storage. Powered by Rust + WASM
via [`mp4-bmff-rs`](https://github.com/Kato-emb/mp4-bmff-rs).

## Features

- **Streaming**: feed one fragment at a time; peak memory is bounded by the
  largest single fragment, not the total file size.
- **OPFS-backed**: writes go through `FileSystemSyncAccessHandle` so the
  output can exceed the heap budget.
- **>4 GiB output**: uses BMFF `largesize` (64-bit) headers throughout.
- **Tiny runtime**: ~360 KiB (release, gzip).

## Requirements

- Modern browser with OPFS `FileSystemSyncAccessHandle` (Chrome / Edge ≥ 102,
  Safari ≥ 17, Firefox ≥ 111).
- A bundler that supports WASM imports (Vite, Webpack 5+, Rollup with
  `@rollup/plugin-wasm`, esbuild ≥ 0.17). For direct browser use without a
  bundler, see the `/web` subpath.
- Must run inside a **Dedicated Web Worker** because
  `FileSystemSyncAccessHandle` is exposed only there.

## Install

This package is published to **GitHub Packages**, so consumers must point npm
at the GitHub registry for the `@kato-emb` scope and authenticate with a
GitHub Personal Access Token (PAT) that has `read:packages` permission.

Project-local `.npmrc`:

```ini
@kato-emb:registry=https://npm.pkg.github.com
//npm.pkg.github.com/:_authToken=${GITHUB_TOKEN}
```

Then:

```bash
GITHUB_TOKEN=<your-pat> npm install @kato-emb/mp4-bmff-wasm
# or, with the token already in the env:
pnpm add @kato-emb/mp4-bmff-wasm
yarn add @kato-emb/mp4-bmff-wasm
```

See <https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-npm-registry#installing-a-package>
for the full GitHub Packages installation flow.

## API

Four functions, called in order from a Worker:

```ts
function initialize(handle: FileSystemSyncAccessHandle): void;
function process_init_segment_from_u8_array(data: Uint8Array): void;
function process_media_segment_from_u8_array(data: Uint8Array): void;
function finalize(date?: Date | null): void;
```

`finalize`'s optional `Date` becomes the resulting `mvhd` / `tkhd`
modification time. The original creation time decoded from the init segment
is preserved when present.

## Usage

### Bundler (Vite / Webpack / Rollup)

`worker.ts`:

```ts
import {
  initialize,
  process_init_segment_from_u8_array,
  process_media_segment_from_u8_array,
  finalize,
} from "@kato-emb/mp4-bmff-wasm";

self.addEventListener("message", async (ev) => {
  const { initBytes, fragmentUrls } = ev.data;

  const opfsRoot = await navigator.storage.getDirectory();
  const file = await opfsRoot.getFileHandle("out.mp4", { create: true });
  const sync = await file.createSyncAccessHandle();

  try {
    initialize(sync);
    process_init_segment_from_u8_array(new Uint8Array(initBytes));
    for (const url of fragmentUrls) {
      const bytes = new Uint8Array(await (await fetch(url)).arrayBuffer());
      process_media_segment_from_u8_array(bytes);
    }
    finalize(new Date());
  } finally {
    sync.close();
  }

  const out = await file.getFile();
  self.postMessage(await out.arrayBuffer(), [await out.arrayBuffer()]);
});
```

`main.ts`:

```ts
const worker = new Worker(new URL("./worker.ts", import.meta.url), {
  type: "module",
});
worker.postMessage({ initBytes, fragmentUrls });
worker.addEventListener("message", (ev) => {
  const blob = new Blob([ev.data], { type: "video/mp4" });
  document.querySelector("a")!.href = URL.createObjectURL(blob);
});
```

### Browser without a bundler (`/web` subpath)

The `/web` subpath ships the `wasm-bindgen --target web` build, which uses an
explicit `init()` call and a fetched `.wasm` URL:

```ts
import init, {
  initialize,
  process_init_segment_from_u8_array,
  process_media_segment_from_u8_array,
  finalize,
} from "@kato-emb/mp4-bmff-wasm/web";

await init(); // fetches the sibling .wasm file
// ...same as above
```

You'll need to host the package's `dist-web/mp4_bmff_wasm_bg.wasm` next to the
JS module, or pass an explicit URL: `await init({ module_or_path: "/path/to/mp4_bmff_wasm_bg.wasm" });`.

### Parallel fragment fetching

Network is the bottleneck — local processing is ~2 GiB/s. Fetch fragments in
parallel and reorder before feeding them to `process_media_segment`:

```ts
async function streamingRemux(urls: string[], concurrency = 6) {
  // ... initialize() etc

  let nextToProcess = 0;
  const ready = new Map<number, Uint8Array>();

  async function fetchAndQueue(i: number, url: string) {
    const buf = new Uint8Array(await (await fetch(url)).arrayBuffer());
    ready.set(i, buf);
    while (ready.has(nextToProcess)) {
      process_media_segment_from_u8_array(ready.get(nextToProcess)!);
      ready.delete(nextToProcess);
      nextToProcess++;
    }
  }

  for (let i = 0; i < urls.length; i += concurrency) {
    await Promise.all(
      urls.slice(i, i + concurrency).map((u, j) => fetchAndQueue(i + j, u)),
    );
  }

  finalize(new Date());
}
```

## Building from source

The published package ships pre-built WASM, but if you cloned the repo and
want to rebuild:

```bash
cd packages/mp4-bmff-wasm
./build.sh release   # generates dist/ and dist-web/
```

Requires `wasm-bindgen-cli` matching `Cargo.lock`'s version.

## Publishing

The package is published to **GitHub Packages** under the `@kato-emb` scope.

### Automated (recommended)

`.github/workflows/publish-wasm.yml` publishes on every push of a tag named
`wasm-vX.Y.Z`. The workflow uses the auto-provisioned `GITHUB_TOKEN` —
**no manual secret setup is required**; the `permissions.packages: write`
directive in the workflow file is enough.

1. Bump `version` in `packages/mp4-bmff-wasm/package.json`.
2. Commit, tag, and push:

   ```bash
   git commit -am "release: mp4-bmff-wasm v0.1.1"
   git tag wasm-v0.1.1
   git push origin main wasm-v0.1.1
   ```

The workflow:

- verifies that the tag (`wasm-v0.1.1`) matches `package.json` (`0.1.1`)
- builds in release with the wasm-bindgen version pinned to `Cargo.lock`
- runs `npm pack` and uploads the tarball as a build artifact
- runs `npm publish` against `https://npm.pkg.github.com`

For a build-only dry run, trigger the workflow manually from the Actions tab
with `dry_run: true` (the default).

### Manual

```bash
cd packages/mp4-bmff-wasm
./build.sh release

# Authenticate to GitHub Packages (one-time, store a classic PAT with
# write:packages permission):
echo "//npm.pkg.github.com/:_authToken=${GITHUB_TOKEN}" >> ~/.npmrc

npm pack --dry-run     # verify the file list
npm publish            # uses publishConfig.registry from package.json
```

### Switching to public npm

To publish to the public npm registry instead, change in `package.json`:

```json
"publishConfig": {
  "access": "public",
  "registry": "https://registry.npmjs.org"
}
```

…and update the workflow's `registry-url` to `https://registry.npmjs.org`,
swap `GITHUB_TOKEN` for an `NPM_TOKEN` repo secret, and add `--provenance`
back to the `npm publish` invocation.

## License

MIT OR Apache-2.0
