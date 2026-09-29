import { describe, expect, it } from 'vitest'
import { safeRedirect } from '~/utils/redirect'

describe('safeRedirect', () => {
  it('同じサイトのパスはそのまま返す', () => {
    expect(safeRedirect('/library/albums')).toBe('/library/albums')
    expect(safeRedirect('/search?q=a#b')).toBe('/search?q=a#b')
  })

  it('別のサイトへの URL はホームにする', () => {
    expect(safeRedirect('https://example.com/')).toBe('/')
    expect(safeRedirect('//example.com/')).toBe('/')
    expect(safeRedirect('/\\example.com/')).toBe('/')
    expect(safeRedirect('javascript:alert(1)')).toBe('/')
  })

  it('ログインの画面と、文字列でない値はホームにする', () => {
    expect(safeRedirect('/login')).toBe('/')
    expect(safeRedirect('/login?redirect=/')).toBe('/')
    expect(safeRedirect(undefined)).toBe('/')
    expect(safeRedirect(['/library'])).toBe('/')
  })

  it('ログインで始まる別の画面は返す', () => {
    expect(safeRedirect('/loginfo')).toBe('/loginfo')
  })
})
