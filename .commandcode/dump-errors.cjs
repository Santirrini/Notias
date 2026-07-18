// Minimal CDP client to dump console messages and JS errors from a headless Chrome.
// Spawns chrome with --remote-debugging-port, then opens the page and prints everything.
const { spawn } = require("node:child_process");
const http = require("node:http");

const CHROME = "C:/Program Files/Google/Chrome/Application/chrome.exe";
const URL = process.argv[2] || "http://127.0.0.1:1420/";
const PORT = 9222;

function get(path) {
  return new Promise((resolve, reject) => {
    http.get(`http://127.0.0.1:${PORT}${path}`, (res) => {
      let buf = "";
      res.on("data", (c) => (buf += c));
      res.on("end", () => resolve(buf));
    }).on("error", reject);
  });
}

async function wait(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

(async () => {
  const chrome = spawn(
    CHROME,
    [
      "--headless=new",
      "--disable-gpu",
      "--no-sandbox",
      `--remote-debugging-port=${PORT}`,
      "--remote-debugging-address=127.0.0.1",
      "--user-data-dir=C:/temp/cdp-profile",
      "about:blank",
    ],
    { stdio: "ignore", windowsHide: true },
  );
  chrome.unref();

  // Wait for DevTools endpoint
  for (let i = 0; i < 30; i++) {
    try {
      const v = await get("/json/version");
      if (v) break;
    } catch {}
    await wait(200);
  }

  const tabs = JSON.parse(await get("/json"));
  const tab = tabs.find((t) => t.type === "page");
  const ws = tab.webSocketDebuggerUrl;
  const WebSocket = require("ws");
  const sock = new WebSocket(ws);
  let id = 0;
  const pending = new Map();
  sock.on("message", (raw) => {
    const msg = JSON.parse(raw.toString());
    if (msg.id && pending.has(msg.id)) {
      pending.get(msg.id)(msg);
      pending.delete(msg.id);
      return;
    }
    if (msg.method) {
      const m = msg.method;
      const p = msg.params || {};
      if (m === "Runtime.consoleAPICalled") {
        const args = (p.args || []).map((a) => a.value ?? a.description ?? "").join(" ");
        console.log(`[${p.type}] ${args}`);
      } else if (m === "Runtime.exceptionThrown") {
        const e = p.exceptionDetails;
        console.log(`[exception] ${e.text}: ${e.exception?.description ?? e.exception?.value ?? ""}`);
      } else if (m === "Log.entryAdded") {
        const e = p.entry;
        if (e.level === "error" || e.level === "warning")
          console.log(`[log:${e.level}] ${e.source}: ${e.text}`);
      } else if (m === "Network.loadingFailed") {
        console.log(`[net:fail] ${p.errorText} requestId=${p.requestId}`);
      } else if (m === "Network.responseReceived") {
        if (p.response.status >= 400)
          console.log(`[net:${p.response.status}] ${p.response.url}`);
      }
    }
  });
  await new Promise((r) => sock.on("open", r));

  function send(method, params) {
    const reqId = ++id;
    return new Promise((resolve) => {
      pending.set(reqId, resolve);
      sock.send(JSON.stringify({ id: reqId, method, params }));
    });
  }

  await send("Runtime.enable");
  await send("Log.enable");
  await send("Network.enable");
  await send("Page.enable");
  await send("Page.navigate", { url: URL });
  await wait(7000);

  // Dump DOM after navigation
  const dom = await send("Runtime.evaluate", {
    expression: "document.body.innerText.slice(0, 500)",
    returnByValue: true,
  });
  console.log("--- BODY innerText ---");
  console.log(dom.result?.result?.value || "(empty)");

  const html = await send("Runtime.evaluate", {
    expression: "document.body.innerHTML.length",
    returnByValue: true,
  });
  console.log("--- body.innerHTML.length =", html.result?.result?.value);

  sock.close();
  chrome.kill();
})().catch((e) => {
  console.error("fatal:", e);
  process.exit(1);
});