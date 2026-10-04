import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'
import license from 'rollup-plugin-license'
import type { Dependency } from 'rollup-plugin-license'

const require = createRequire(import.meta.url)

interface Entry {
  name: string
  version: string
  license: string
  url: string | null
  text: string
  notice: string | null
}

/** package.json の repository は `owner/repo` や `git+https://…git` とも書けるので、開ける URL に直す。 */
function repositoryUrl(repository: Dependency['repository']): string | null {
  const raw = typeof repository === 'string' ? repository : repository?.url
  if (!raw) {
    return null
  }
  if (/^[\w.-]+\/[\w.-]+$/.test(raw)) {
    return `https://github.com/${raw}`
  }
  return raw.replace(/^git\+/, '').replace(/^git:\/\//, 'https://').replace(/\.git$/, '')
}

function fromDependency(dep: Dependency): Entry {
  return {
    name: dep.name ?? '',
    version: dep.version ?? '',
    license: dep.license ?? '',
    url: dep.homepage ?? repositoryUrl(dep.repository),
    text: dep.licenseText ?? '',
    notice: dep.noticeText,
  }
}

/** CSS から読むフォントはバンドルの依存に現れないので、パッケージから直接読む。 */
function fromPackage(name: string): Entry {
  const pkg = require(`${name}/package.json`)
  return {
    name,
    version: pkg.version,
    license: pkg.license,
    url: pkg.homepage ?? null,
    text: readFileSync(require.resolve(`${name}/LICENSE`), 'utf8'),
    notice: null,
  }
}

function render(entries: Entry[]): string {
  const sections = entries
    .sort((a, b) => a.name.localeCompare(b.name))
    .map((e) => {
      const title = e.url ? `[${e.name} ${e.version}](${e.url})` : `${e.name} ${e.version}`
      const notice = e.notice ? `\n\n~~~~text\n${e.notice.trim()}\n~~~~` : ''
      return `## ${title}\n\n${e.license}\n\n~~~~text\n${e.text.trim()}\n~~~~${notice}\n`
    })
  return [
    '# Web の依存のライセンス',
    '',
    'Web のビルド結果に入る依存と、そのライセンス。',
    '`web/` で `pnpm build` を実行すると作り直される。手で書き換えない。',
    '',
    ...sections,
  ].join('\n')
}

/** Web のビルド結果に入る依存のライセンスを THIRD_PARTY_LICENSES.md に書き出す。 */
export function thirdPartyLicenses() {
  return license({
    thirdParty: {
      // 許すライセンス以外の依存がバンドルに入ったらビルドを落とす（docs/web.md の「依存の照合」）。
      // 「MIT か GPL」のように選べるものは、許すほうを選べるので通る
      allow: {
        test: '(MIT OR Apache-2.0 OR ISC)',
        failOnUnlicensed: true,
        failOnViolation: true,
      },
      output: {
        file: fileURLToPath(new URL('../THIRD_PARTY_LICENSES.md', import.meta.url)),
        template: deps => render([
          ...deps.map(fromDependency),
          fromPackage('@fontsource-variable/geist'),
        ]),
      },
    },
  })
}
