// Main thread: directory picker + worker orchestration + download UI.
//
// The actual fMP4 → MP4 work happens inside ./worker.js because the OPFS
// FileSystemSyncAccessHandle API is only available in DedicatedWorkerGlobalScope.

const INIT_EXTS = new Set(["mp4", "cmfi", "m4i"]);
const FRAGMENT_EXTS = new Set([
  "m4s",
  "m4v",
  "m4a",
  "cmfv",
  "cmfa",
  "cmft",
]);

const $ = (id) => document.getElementById(id);
const dirInput = $("dir");
const fileList = $("filelist");
const countEl = $("count");
const runBtn = $("run");
const statusEl = $("status");
const resultEl = $("result");
const logEl = $("log");

let initFile = null;
let fragmentFiles = [];

function log(msg) {
  const stamp = new Date().toLocaleTimeString();
  logEl.textContent += `[${stamp}] ${msg}\n`;
  logEl.scrollTop = logEl.scrollHeight;
}

function extOf(name) {
  const i = name.lastIndexOf(".");
  return i < 0 ? "" : name.slice(i + 1).toLowerCase();
}

dirInput.addEventListener("change", () => {
  initFile = null;
  fragmentFiles = [];
  fileList.innerHTML = "";
  resultEl.textContent = "未実行";
  runBtn.disabled = true;

  const files = Array.from(dirInput.files || []);
  if (files.length === 0) {
    countEl.hidden = true;
    return;
  }

  const initCandidates = [];
  for (const f of files) {
    const ext = extOf(f.name);
    if (INIT_EXTS.has(ext)) {
      initCandidates.push(f);
    } else if (FRAGMENT_EXTS.has(ext)) {
      fragmentFiles.push(f);
    }
  }

  if (initCandidates.length === 0) {
    log(
      `init segment が見つかりません (許容拡張子: ${[...INIT_EXTS].join(", ")})`,
    );
    countEl.hidden = true;
    return;
  }
  if (initCandidates.length > 1) {
    log(
      `init segment 候補が複数あります: ${initCandidates
        .map((f) => f.name)
        .join(", ")}`,
    );
    countEl.hidden = true;
    return;
  }
  initFile = initCandidates[0];

  // sort fragments by lexicographic name (matches the CLI example)
  fragmentFiles.sort((a, b) => a.name.localeCompare(b.name));

  countEl.hidden = false;
  countEl.textContent = `init: 1, fragments: ${fragmentFiles.length}`;

  const initLi = document.createElement("li");
  initLi.className = "init";
  initLi.textContent = `init: ${initFile.webkitRelativePath || initFile.name}`;
  fileList.appendChild(initLi);
  for (const f of fragmentFiles) {
    const li = document.createElement("li");
    li.textContent = f.webkitRelativePath || f.name;
    fileList.appendChild(li);
  }

  runBtn.disabled = fragmentFiles.length === 0;
});

runBtn.addEventListener("click", async () => {
  runBtn.disabled = true;
  statusEl.textContent = "処理中...";
  resultEl.textContent = "処理中...";
  log("run start");

  const worker = new Worker(new URL("./worker.js", import.meta.url), {
    type: "module",
  });

  worker.addEventListener("message", (ev) => {
    const msg = ev.data;
    switch (msg.kind) {
      case "log":
        log("worker: " + msg.message);
        break;
      case "progress":
        statusEl.textContent = msg.message;
        break;
      case "done": {
        const totalMs = msg.timings?.total_ms ?? 0;
        statusEl.textContent = `完了 (${fmtMs(totalMs)})`;
        const url = URL.createObjectURL(msg.blob);
        const filename = msg.filename || "output.mp4";
        resultEl.innerHTML = "";

        const a = document.createElement("a");
        a.className = "download";
        a.href = url;
        a.download = filename;
        a.textContent = `${filename} をダウンロード (${msg.blob.size.toLocaleString()} bytes)`;
        resultEl.appendChild(a);

        if (msg.timings) {
          resultEl.appendChild(renderTimings(msg.timings, msg.blob.size));
        }

        log(`done: ${msg.blob.size} bytes in ${fmtMs(totalMs)}`);
        worker.terminate();
        runBtn.disabled = false;
        break;
      }
      case "error":
        statusEl.textContent = "エラー";
        resultEl.textContent = "失敗: " + msg.message;
        log("ERROR: " + msg.message);
        if (msg.stack) log(msg.stack);
        worker.terminate();
        runBtn.disabled = false;
        break;
    }
  });

  worker.addEventListener("error", (ev) => {
    statusEl.textContent = "Worker 起動エラー";
    log(`worker error: ${ev.message}`);
    runBtn.disabled = false;
  });

  // Read all files as ArrayBuffers up front and transfer ownership to the
  // worker. We ship one fragment per call so peak memory inside the worker
  // is bounded by the largest single fragment.
  try {
    const initBuf = await initFile.arrayBuffer();
    const fragmentBufs = [];
    for (const f of fragmentFiles) {
      fragmentBufs.push({
        name: f.name,
        bytes: await f.arrayBuffer(),
      });
    }

    const transferable = [initBuf, ...fragmentBufs.map((f) => f.bytes)];
    worker.postMessage(
      {
        kind: "run",
        init: { name: initFile.name, bytes: initBuf },
        fragments: fragmentBufs,
        outputFilename: deriveOutputFilename(initFile, fragmentFiles),
      },
      transferable,
    );
  } catch (e) {
    statusEl.textContent = "読み込み失敗";
    log("read error: " + e.message);
    worker.terminate();
    runBtn.disabled = false;
  }
});

function fmtMs(ms) {
  if (ms < 1) return `${(ms * 1000).toFixed(0)}μs`;
  if (ms < 1000) return `${ms.toFixed(1)}ms`;
  return `${(ms / 1000).toFixed(2)}s`;
}

function fmtBytes(n) {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KiB`;
  return `${(n / 1024 / 1024).toFixed(2)} MiB`;
}

function renderTimings(t, outputBytes) {
  const table = document.createElement("table");
  table.style.cssText =
    "margin-top:0.75rem; border-collapse:collapse; font-size:0.85rem;";
  const rows = [
    ["WASM 初期化", fmtMs(t.wasm_init_ms)],
    ["init segment 処理", fmtMs(t.init_seg_ms)],
    [
      `fragments (${t.fragments_count} 個, ${fmtBytes(t.fragments_total_bytes)})`,
      `${fmtMs(t.fragments_ms)} (avg ${fmtMs(t.fragments_avg_ms)}/個)`,
    ],
  ];
  if (t.max_fragment_name) {
    rows.push([
      "  最遅 fragment",
      `${t.max_fragment_name} (${fmtMs(t.max_fragment_ms)})`,
    ]);
  }
  rows.push(["finalize (moov + mdat header)", fmtMs(t.finalize_ms)]);
  rows.push(["OPFS read-back", fmtMs(t.readback_ms)]);

  if (t.fragments_ms > 0 && t.fragments_total_bytes > 0) {
    const mibs =
      t.fragments_total_bytes / 1024 / 1024 / (t.fragments_ms / 1000);
    rows.push(["fragment スループット", `${mibs.toFixed(1)} MiB/s`]);
  }
  if (t.total_ms > 0 && outputBytes > 0) {
    const overall = outputBytes / 1024 / 1024 / (t.total_ms / 1000);
    rows.push(["全体スループット (output / total)", `${overall.toFixed(1)} MiB/s`]);
  }

  rows.push([
    `合計`,
    `${fmtMs(t.total_ms)}`,
  ]);

  for (const [label, value] of rows) {
    const tr = document.createElement("tr");
    const td1 = document.createElement("td");
    const td2 = document.createElement("td");
    td1.textContent = label;
    td2.textContent = value;
    td1.style.cssText = "padding:2px 12px 2px 0; color:#555;";
    td2.style.cssText =
      "padding:2px 0; font-family:ui-monospace,monospace; text-align:right;";
    if (label === "合計") {
      td1.style.fontWeight = "600";
      td2.style.fontWeight = "600";
    }
    tr.appendChild(td1);
    tr.appendChild(td2);
    table.appendChild(tr);
  }
  return table;
}

function deriveOutputFilename(initFile, _fragments) {
  const root = initFile.webkitRelativePath
    ? initFile.webkitRelativePath.split("/")[0]
    : null;
  return (root || "output") + ".mp4";
}
