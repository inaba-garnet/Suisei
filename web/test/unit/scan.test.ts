import { describe, expect, it } from 'vitest'
import { scanReportMessage } from '~/utils/scan'

const NOW = 1_700_000_000_000

describe('scanReportMessage', () => {
  it('件数か、失敗の理由を出す', () => {
    expect(scanReportMessage({ at: NOW - 5 * 60_000, full: false, elapsedMs: 1000, files: 3066, read: 12, failed: 0 }, NOW))
      .toBe('5 分前のスキャン: 3,066 ファイル、読み直し 12 件')
    expect(scanReportMessage({ at: NOW, full: true, elapsedMs: 1000, error: '音楽フォルダを読めない' }, NOW))
      .toBe('たった今の全ファイルの読み直しは失敗しました（音楽フォルダを読めない）')
  })
})
