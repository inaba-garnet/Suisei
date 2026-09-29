import type { ThemePreference } from '~/utils/theme'
import { parseThemePreference, resolveTheme, THEME_STORAGE_KEY } from '~/utils/theme'

/** テーマの選択。OS の設定が変わったら、`system` のときに追従する（docs/web.md）。 */
export function useTheme() {
  const preference = useState<ThemePreference>('theme', () => parseThemePreference(readStorage()))
  const media = import.meta.client ? window.matchMedia('(prefers-color-scheme: light)') : undefined

  function apply() {
    const theme = resolveTheme(preference.value, media?.matches ?? false)
    const root = document.documentElement
    root.dataset.theme = theme
    // スマホのブラウザのバーの色を、背景に合わせる
    const background = getComputedStyle(root).getPropertyValue('--surface-0').trim()
    document.querySelector('meta[name="theme-color"]')?.setAttribute('content', background)
  }

  function setPreference(value: ThemePreference) {
    preference.value = value
    try {
      if (value === 'system') {
        localStorage.removeItem(THEME_STORAGE_KEY)
      }
      else {
        localStorage.setItem(THEME_STORAGE_KEY, value)
      }
    }
    catch {
      // プライベートブラウズなどで保存できなくても、この画面の間は切り替える
    }
    apply()
  }

  return { preference, setPreference, apply, media }
}

function readStorage(): string | null {
  try {
    return localStorage.getItem(THEME_STORAGE_KEY)
  }
  catch {
    return null
  }
}
