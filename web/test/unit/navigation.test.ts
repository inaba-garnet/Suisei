import { describe, expect, it } from 'vitest'
import { isUnder, mainNav } from '~/utils/navigation'

describe('mainNav', () => {
  it('下のタブは五つ', () => {
    expect(mainNav.map(item => item.label)).toEqual(['ホーム', '検索', 'ライブラリ', 'プレイリスト', '設定'])
  })
})

describe('isUnder', () => {
  it('その画面と、その下の画面を含む', () => {
    expect(isUnder('/library', '/library')).toBe(true)
    expect(isUnder('/library/albums', '/library')).toBe(true)
  })

  it('名前が前方一致するだけの画面は含まない', () => {
    expect(isUnder('/libraryx', '/library')).toBe(false)
  })

  it('ホームはホームだけを指す', () => {
    expect(isUnder('/', '/')).toBe(true)
    expect(isUnder('/search', '/')).toBe(false)
  })
})
