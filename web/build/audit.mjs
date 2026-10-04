// 依存の脆弱性を照らす（docs/web.md の「依存の照合」）。
// バンドルに入る依存（THIRD_PARTY_LICENSES.md に載るもの）の脆弱性だけで落とし、
// ビルドの道具や開発サーバーの脆弱性は警告に留める。後者はブラウザに配られないため。
import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'

const licenses = readFileSync(new URL('../THIRD_PARTY_LICENSES.md', import.meta.url), 'utf8')
const bundled = new Set(
  [...licenses.matchAll(/^## \[?(\S+) \S+?\]?(?:\(|$)/gm)].map(m => m[1]),
)

let report
try {
  report = execFileSync('pnpm', ['audit', '--prod', '--json'], { encoding: 'utf8' })
}
catch (e) {
  // 脆弱性があると終了コードが 0 以外になるので、出力だけ使う
  report = e.stdout
}
const advisories = Object.values(JSON.parse(report).advisories ?? {})

let failed = false
for (const a of advisories) {
  const message = `${a.module_name} ${a.vulnerable_versions}（${a.severity}）: ${a.title} ${a.url}`
  if (bundled.has(a.module_name)) {
    console.log(`::error::バンドルに入る依存: ${message}`)
    failed = true
  }
  else {
    console.log(`::warning::バンドルの外の依存: ${message}`)
  }
}
console.log(`脆弱性 ${advisories.length} 件（バンドルに入る依存の照合対象 ${bundled.size} 件）`)
process.exit(failed ? 1 : 0)
