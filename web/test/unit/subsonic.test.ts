import { describe, expect, it } from 'vitest'
import { gridColumns } from '~/utils/grid'
import { coverArtUrl, formatDuration, SubsonicError, unwrap } from '~/utils/subsonic'

describe('unwrap', () => {
  it('成功の応答から中身を取り出す', () => {
    const body = { 'subsonic-response': { status: 'ok', albumList2: { album: [] } } }
    expect(unwrap<{ albumList2: unknown }>(body).albumList2).toEqual({ album: [] })
  })

  it('失敗の応答はエラーコードを持つ SubsonicError にする', () => {
    const body = { 'subsonic-response': { status: 'failed', error: { code: 40, message: 'wrong' } } }
    expect(() => unwrap(body)).toThrow(SubsonicError)
    try {
      unwrap(body)
    }
    catch (err) {
      expect((err as SubsonicError).code).toBe(40)
    }
  })

  it('形の違う応答もエラーにする', () => {
    expect(() => unwrap({})).toThrow(SubsonicError)
    expect(() => unwrap(null)).toThrow(SubsonicError)
  })
})

describe('coverArtUrl', () => {
  it('ID と大きさを引数に入れる', () => {
    const url = new URL(coverArtUrl('al 1', 300), 'http://x')
    expect(url.pathname).toBe('/rest/getCoverArt')
    expect(url.searchParams.get('id')).toBe('al 1')
    expect(url.searchParams.get('size')).toBe('300')
  })
})

describe('formatDuration', () => {
  it('分と秒、1 時間を超えれば時も出す', () => {
    expect(formatDuration(0)).toBe('0:00')
    expect(formatDuration(185)).toBe('3:05')
    expect(formatDuration(3723)).toBe('1:02:03')
  })
})

describe('gridColumns', () => {
  it('内容の幅に合わせて 2〜6 列にする', () => {
    expect(gridColumns(360)).toBe(2)
    expect(gridColumns(640)).toBe(4)
    expect(gridColumns(860)).toBe(5)
    expect(gridColumns(1040)).toBe(6)
  })
})
