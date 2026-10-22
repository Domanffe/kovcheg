import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import { URL } from "node:url";
import { allSourceMeta } from "../sources/meta";
import { runConcurrentSearch, type ConcurrentSearchState } from "../core/search";

const DEFAULT_PORT = 3847;

function requestedPort(): number {
  const raw = process.argv[2] ?? process.env.KOVCHEG_SEARCH_PORT ?? String(DEFAULT_PORT);
  const port = Number(raw);
  return Number.isInteger(port) && port >= 0 ? port : DEFAULT_PORT;
}

function sendJson(res: ServerResponse, status: number, body: unknown): void {
  res.writeHead(status, { "Content-Type": "application/json", "Access-Control-Allow-Origin": "*" });
  res.end(JSON.stringify(body));
}

function handleHealth(res: ServerResponse): void {
  sendJson(res, 200, { ok: true });
}

function handleSources(res: ServerResponse): void {
  sendJson(res, 200, { sources: allSourceMeta() });
}

function handleSearch(req: IncomingMessage, res: ServerResponse): void {
  const url = new URL(req.url ?? "/", `http://${req.headers.host ?? "localhost"}`);
  const query = url.searchParams.get("q") ?? "";

  res.writeHead(200, {
    "Content-Type": "text/event-stream",
    "Cache-Control": "no-cache",
    Connection: "keep-alive",
    "Access-Control-Allow-Origin": "*",
  });

  const ctrl = new AbortController();
  req.on("close", () => ctrl.abort());

  const write = (state: ConcurrentSearchState): void => {
    res.write(`data: ${JSON.stringify(state)}\n\n`);
  };

  void runConcurrentSearch(query, { onUpdate: write }, ctrl.signal)
    .then(() => {
      res.write("event: done\ndata: {}\n\n");
      res.end();
    })
    .catch((e: unknown) => {
      const msg = e instanceof Error ? e.message : String(e);
      res.write(`event: error\ndata: ${JSON.stringify({ error: msg })}\n\n`);
      res.end();
    });
}

export function startSearchServer(port = DEFAULT_PORT): Promise<{ port: number; close: () => void }> {
  return new Promise((resolve, reject) => {
    const server = createServer((req, res) => {
      const path = (req.url ?? "/").split("?")[0];
      if (req.method === "GET" && path === "/health") return handleHealth(res);
      if (req.method === "GET" && path === "/sources") return handleSources(res);
      if (req.method === "GET" && path === "/search") return handleSearch(req, res);
      sendJson(res, 404, { error: "not found" });
    });

    server.on("error", reject);
    server.listen(port, "127.0.0.1", () => {
      const addr = server.address();
      const actual = typeof addr === "object" && addr ? addr.port : port;
      process.stdout.write(`search-sidecar-port:${actual}\n`);
      resolve({
        port: actual,
        close: () => server.close(),
      });
    });
  });
}

if (import.meta.url === `file://${process.argv[1]?.replace(/\\/g, "/")}` || process.argv[1]?.endsWith("search-server.js")) {
  void startSearchServer(requestedPort());
}
