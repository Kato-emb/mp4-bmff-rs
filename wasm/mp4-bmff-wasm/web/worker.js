// Worker thread: loads the WASM module and drives the assembler.
//
// FileSystemSyncAccessHandle is only exposed inside DedicatedWorkerGlobalScope,
// so all OPFS-touching code lives here.

import init, {
  initialize,
  process_init_segment_from_u8_array,
  process_media_segment_from_u8_array,
  finalize,
} from "./pkg/mp4_bmff_wasm.js";

const post = (msg, transfer) => self.postMessage(msg, transfer || []);
const log = (m) => post({ kind: "log", message: m });
const progress = (m) => post({ kind: "progress", message: m });
const fmt_ms = (ms) =>
  ms < 1 ? `${(ms * 1000).toFixed(0)}μs` : `${ms.toFixed(1)}ms`;

self.addEventListener("message", async (ev) => {
  const msg = ev.data;
  if (msg.kind !== "run") return;

  const t = (() => {
    const marks = {};
    const t0 = performance.now();
    return {
      mark(name) {
        marks[name] = performance.now();
      },
      since(name) {
        return performance.now() - marks[name];
      },
      total() {
        return performance.now() - t0;
      },
    };
  })();

  try {
    t.mark("wasm_init");
    progress("WASM 初期化");
    await init();
    const wasm_init_ms = t.since("wasm_init");
    log(`wasm loaded (${fmt_ms(wasm_init_ms)})`);

    // Create a temp file in OPFS that we will write into. Use a unique name so
    // multiple runs in the same origin don't clash.
    t.mark("opfs_open");
    const opfsRoot = await navigator.storage.getDirectory();
    const tmpName = `fmp4-asm-${Date.now()}.tmp`;
    const fileHandle = await opfsRoot.getFileHandle(tmpName, { create: true });
    const sync = await fileHandle.createSyncAccessHandle();
    log(`OPFS file: ${tmpName} (${fmt_ms(t.since("opfs_open"))})`);

    let init_seg_ms = 0;
    let fragments_ms = 0;
    let finalize_ms = 0;
    let total_fragment_bytes = 0;
    let max_fragment_ms = 0;
    let max_fragment_name = "";

    try {
      // 1) bind handle, 2) feed init, 3) feed each fragment, 4) finalize
      progress("初期化セグメント処理");
      t.mark("init_seg");
      initialize(sync);
      process_init_segment_from_u8_array(new Uint8Array(msg.init.bytes));
      init_seg_ms = t.since("init_seg");
      log(
        `init segment: ${msg.init.name} (${msg.init.bytes.byteLength} B, ${fmt_ms(init_seg_ms)})`,
      );

      const total = msg.fragments.length;
      t.mark("fragments");
      for (let i = 0; i < total; i++) {
        const f = msg.fragments[i];
        progress(`fragment ${i + 1}/${total}: ${f.name}`);
        const t0 = performance.now();
        process_media_segment_from_u8_array(new Uint8Array(f.bytes));
        const dt = performance.now() - t0;
        total_fragment_bytes += f.bytes.byteLength;
        if (dt > max_fragment_ms) {
          max_fragment_ms = dt;
          max_fragment_name = f.name;
        }
      }
      fragments_ms = t.since("fragments");
      const avg_ms = total > 0 ? fragments_ms / total : 0;
      const throughput_mbps =
        fragments_ms > 0
          ? (total_fragment_bytes / 1024 / 1024) / (fragments_ms / 1000)
          : 0;
      log(
        `processed ${total} fragments (${(total_fragment_bytes / 1024 / 1024).toFixed(2)} MiB) in ${fmt_ms(fragments_ms)}` +
          ` — avg ${fmt_ms(avg_ms)}/fragment, throughput ${throughput_mbps.toFixed(1)} MiB/s`,
      );
      if (max_fragment_name) {
        log(
          `  slowest fragment: ${max_fragment_name} (${fmt_ms(max_fragment_ms)})`,
        );
      }

      progress("finalize");
      t.mark("finalize");
      finalize(new Date());
      finalize_ms = t.since("finalize");
      log(`finalize done (${fmt_ms(finalize_ms)})`);
    } finally {
      // SyncAccessHandle must be closed before the file can be read back.
      sync.close();
    }

    // Read the resulting file back as a Blob and ship it to the main thread.
    t.mark("readback");
    progress("結果を読み出し中");
    const file = await fileHandle.getFile();
    const blob = new Blob([await file.arrayBuffer()], { type: "video/mp4" });
    const readback_ms = t.since("readback");
    log(
      `readback: ${(blob.size / 1024 / 1024).toFixed(2)} MiB (${fmt_ms(readback_ms)})`,
    );

    // Clean up the OPFS scratch file. (Comment out if you'd rather inspect it
    // via DevTools → Application → Storage → OPFS.)
    await opfsRoot.removeEntry(tmpName).catch(() => {});

    const total_ms = t.total();
    log(`TOTAL: ${fmt_ms(total_ms)}`);

    post(
      {
        kind: "done",
        blob,
        filename: msg.outputFilename || "output.mp4",
        timings: {
          wasm_init_ms,
          init_seg_ms,
          fragments_ms,
          fragments_avg_ms: msg.fragments.length
            ? fragments_ms / msg.fragments.length
            : 0,
          fragments_count: msg.fragments.length,
          fragments_total_bytes: total_fragment_bytes,
          finalize_ms,
          readback_ms,
          total_ms,
          max_fragment_ms,
          max_fragment_name,
        },
      },
      // Blob can't be transferred; the underlying ArrayBuffer is already
      // copied internally so this list stays empty.
      [],
    );
  } catch (e) {
    post({
      kind: "error",
      message: e?.message || String(e),
      stack: e?.stack,
    });
  }
});
