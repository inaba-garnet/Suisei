import { describe, expect, it } from 'vitest'
import { callbackErrorMessage, formatAgo, lastSyncMessage } from '~/utils/spotify'

const MINUTE = 60_000
const NOW = 1_700_000_000_000

describe('formatAgo', () => {
  it('分、時間、日の単位で丸める', () => {
    expect(formatAgo(NOW - 30_000, NOW)).toBe('たった今')
    expect(formatAgo(NOW - 5 * MINUTE, NOW)).toBe('5 分前')
    expect(formatAgo(NOW - 3 * 60 * MINUTE, NOW)).toBe('3 時間前')
    expect(formatAgo(NOW - 50 * 60 * MINUTE, NOW)).toBe('2 日前')
  })

  it('時計がずれて未来の日時でも、たった今にする', () => {
    expect(formatAgo(NOW + MINUTE, NOW)).toBe('たった今')
  })
})

describe('lastSyncMessage', () => {
  it('取り込んだ曲数か、失敗の理由を出す', () => {
    expect(lastSyncMessage({ at: NOW, error: null, fetched: 10, matched: 3 }, NOW)).toBe('たった今、3 曲をお気に入りにしました')
    expect(lastSyncMessage({ at: NOW, error: null, fetched: null, matched: 0 }, NOW)).toBe('たった今、新しく取り込んだ曲はありませんでした')
    expect(lastSyncMessage({ at: NOW - 5 * MINUTE, error: 'boom', fetched: null, matched: 0 }, NOW)).toBe('5 分前、取り込みに失敗しました（boom）')
  })
})

describe('callbackErrorMessage', () => {
  it('知らない理由は Spotify につながらなかったとみなす', () => {
    expect(callbackErrorMessage('unknownState')).toContain('やり直して')
    expect(callbackErrorMessage(undefined)).toContain('つながりませんでした')
  })
})
