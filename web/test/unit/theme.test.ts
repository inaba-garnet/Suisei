import { describe, expect, it } from 'vitest'
import { parseThemePreference, resolveTheme } from '~/utils/theme'

describe('parseThemePreference', () => {
  it('保存された値を読み、知らない値は OS に従う', () => {
    expect(parseThemePreference('light')).toBe('light')
    expect(parseThemePreference('dark')).toBe('dark')
    expect(parseThemePreference(null)).toBe('system')
    expect(parseThemePreference('blue')).toBe('system')
  })
})

describe('resolveTheme', () => {
  it('system は OS の設定に従う', () => {
    expect(resolveTheme('system', true)).toBe('light')
    expect(resolveTheme('system', false)).toBe('dark')
  })

  it('手動の選択は OS の設定より優先する', () => {
    expect(resolveTheme('dark', true)).toBe('dark')
    expect(resolveTheme('light', false)).toBe('light')
  })
})
