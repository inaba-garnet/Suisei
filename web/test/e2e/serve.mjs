// E2E 用に、書き出した SPA（.output/public）を配る。知らないパスには 200.html を返す
import { createReadStream, statSync } from 'node:fs'
import { createServer } from 'node:http'
import { extname, join, normalize } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../../.output/public', import.meta.url))
const port = Number(process.env.PORT ?? 3100)
const types = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript',
  '.css': 'text/css',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.woff2': 'font/woff2',
  '.json': 'application/json',
}

function resolve(pathname) {
  const path = join(root, normalize(decodeURIComponent(pathname)))
  for (const candidate of [path, join(path, 'index.html')]) {
    try {
      if (candidate.startsWith(root) && statSync(candidate).isFile()) {
        return candidate
      }
    }
    catch {
      // 次の候補を試す
    }
  }
  return join(root, '200.html')
}

createServer((req, res) => {
  const file = resolve(new URL(req.url, 'http://localhost').pathname)
  res.writeHead(200, { 'Content-Type': types[extname(file)] ?? 'application/octet-stream' })
  createReadStream(file).pipe(res)
}).listen(port)
