import { ProxyAgent, setGlobalDispatcher } from "undici";
import { startSearchServer } from "./search-server.js";

// Trackers and repack sites are blocked by many ISPs (and by RU DNS in particular).
// Browsers and curl honour the proxy env vars, Node's fetch ignores them — so adopt them
// here, otherwise every such source reports "fetch failed" while the same URL opens in a browser.
const proxyUrl =
  process.env.KOVCHEG_PROXY ??
  process.env.HTTPS_PROXY ??
  process.env.https_proxy ??
  process.env.HTTP_PROXY ??
  process.env.http_proxy ??
  "";

if (proxyUrl.trim()) {
  try {
    setGlobalDispatcher(new ProxyAgent(proxyUrl.trim()));
    process.stderr.write(`search-sidecar: proxy ${proxyUrl.trim()}\n`);
  } catch (e) {
    process.stderr.write(`search-sidecar: proxy rejected (${String(e)})\n`);
  }
}

// Orphan guard: if the app dies without shutting us down (crash, kill), exit instead of
// sitting in the background holding a port and a copy of the binary. Node can probe a pid on
// Windows too: process.kill(pid, 0) throws when it is gone.
const parentPid = Number(process.env.KOVCHEG_PARENT_PID ?? 0);
if (Number.isInteger(parentPid) && parentPid > 0) {
  setInterval(() => {
    try {
      process.kill(parentPid, 0);
    } catch {
      process.stderr.write("search-sidecar: parent is gone, exiting\n");
      process.exit(0);
    }
  }, 5000).unref();
}

// Port 0 = OS-assigned (production sidecar reports actual port on stdout).
const port = Number(process.env.KOVCHEG_SEARCH_PORT ?? 0);
void startSearchServer(port);
