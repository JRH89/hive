import { createServer } from "node:http";
import { readFile, stat } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { resolve, extname, sep } from "node:path";

const root = resolve(fileURLToPath(new URL("../out/", import.meta.url)));
const types = {
  ".html": "text/html; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".svg": "image/svg+xml",
  ".txt": "text/plain; charset=utf-8",
  ".json": "application/json",
  ".woff2": "font/woff2",
  ".ico": "image/x-icon",
};

try {
  await stat(resolve(root, "index.html"));
} catch {
  console.error("Run npm run build before previewing the static export.");
  process.exit(1);
}

createServer(async (request, response) => {
  try {
    const pathname = decodeURIComponent(
      new URL(request.url ?? "/", "http://localhost").pathname,
    );
    let file = resolve(root, `.${pathname}`);
    if (
      file !== root &&
      !file.startsWith(root.endsWith(sep) ? root : root + sep)
    ) {
      response.writeHead(403).end("Forbidden");
      return;
    }
    if ((await stat(file)).isDirectory()) file = resolve(file, "index.html");
    response
      .writeHead(200, {
        "Content-Type": types[extname(file)] ?? "application/octet-stream",
        "X-Content-Type-Options": "nosniff",
      })
      .end(await readFile(file));
  } catch {
    response
      .writeHead(404, { "Content-Type": "text/plain; charset=utf-8" })
      .end("Not found");
  }
}).listen(4310, "127.0.0.1", () =>
  console.log("Hive website preview: http://127.0.0.1:4310"),
);
