import { createReadStream, statSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, normalize } from "node:path";

const root = join(import.meta.dirname, "..", "desktop", "dist");
const port = Number(process.env.RHO_PREVIEW_PORT || 4173);
const types = {
  ".css": "text/css; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".png": "image/png",
};

createServer((request, response) => {
  const relative = decodeURIComponent((request.url || "/").split("?")[0]);
  const candidate = normalize(join(root, relative === "/" ? "index.html" : relative));
  if (!candidate.startsWith(root)) {
    response.writeHead(403).end("Forbidden");
    return;
  }
  try {
    if (!statSync(candidate).isFile()) throw new Error("not a file");
    response.writeHead(200, { "Content-Type": types[extname(candidate)] || "application/octet-stream" });
    createReadStream(candidate).pipe(response);
  } catch {
    response.writeHead(404).end("Not found");
  }
}).listen(port, "127.0.0.1", () => {
  console.log(`Rho preview: http://127.0.0.1:${port}`);
});
