export type Theme = 'light' | 'dark'
/** 設定の画面で選ぶ値。`system` は OS の設定に従う。 */
export type ThemePreference = Theme | 'system'

export const THEME_STORAGE_KEY = 'suisei-theme'

/** 保存された値を読む。知らない値は OS の設定に従う。 */
export function parseThemePreference(value: string | null): ThemePreference {
  return value === 'light' || value === 'dark' ? value : 'system'
}

export function resolveTheme(preference: ThemePreference, prefersLight: boolean): Theme {
  if (preference === 'system') {
    return prefersLight ? 'light' : 'dark'
  }
  return preference
}
