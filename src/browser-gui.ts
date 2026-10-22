import { spawn } from "node:child_process";
import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import { promises as fs } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { startSearchServer } from "./sidecar/search-server.js";

function openUrl(url: string): void {
  if (process.platform === "win32") spawn("cmd", ["/c", "start", "", url], { detached: true, stdio: "ignore" });
  else if (process.platform === "darwin") spawn("open", [url], { detached: true, stdio: "ignore" });
  else spawn("xdg-open", [url], { detached: true, stdio: "ignore" });
}

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const GUI_ROOT = path.join(__dirname, "..", "dist", "gui");

function contentType(file: string): string {
  if (file.endsWith(".html")) return "text/html";
  if (file.endsWith(".js")) return "text/javascript";
  if (file.endsWith(".css")) return "text/css";
  if (file.endsWith(".svg")) return "image/svg+xml";
  return "application/octet-stream";
}

async function serveStatic(req: IncomingMessage, res: ServerResponse): Promise<void> {
  const urlPath = (req.url ?? "/").split("?")[0]!;
  const rel = urlPath === "/" ? "index.html" : urlPath.replace(/^\//, "");
  const file = path.normalize(path.join(GUI_ROOT, rel));
  if (!file.startsWith(GUI_ROOT)) {
    res.writeHead(403);
    res.end();
    return;
  }
  try {
    const data = await fs.readFile(file);
    res.writeHead(200, { "Content-Type": contentType(file) });
    res.end(data);
  } catch {
    try {
      const fallback = await fs.readFile(path.join(GUI_ROOT, "index.html"));
      res.writeHead(200, { "Content-Type": "text/html" });
      res.end(fallback);
    } catch {
      res.writeHead(404);
      res.end("GUI not built. Run: npm run build:gui");
    }
  }
}

export async function runBrowserGui(): Promise<void> {
  const search = await startSearchServer(0);
  const guiPort = await new Promise<number>((resolve, reject) => {
    const server = createServer((req, res) => {
      void serveStatic(req, res);
    });
    server.on("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const addr = server.address();
      resolve(typeof addr === "object" && addr ? addr.port : 0);
    });
  });

  const url = `http://127.0.0.1:${guiPort}/?searchPort=${search.port}`;
  process.stdout.write(`kovcheg GUI: ${url}\n`);
  process.stdout.write(`search API: http://127.0.0.1:${search.port}\n`);
  openUrl(url);
  process.stdout.write("Press Ctrl+C to stop.\n");
}
