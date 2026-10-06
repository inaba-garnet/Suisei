/** スキャンの状態（`GET /api/scan`、docs/schema.md の「スキャン」）の型と、表示の補助。 */

import { formatAgo } from '~/utils/time'

export interface ScanReport {
  /** 終わった日時（UNIX 時刻のミリ秒） */
  at: number
  /** 全ファイルを読み直したか */
  full: boolean
  elapsedMs: number
  /** 失敗したときの理由。成功したときは件数の項目がある */
  error?: string
  files?: number
  read?: number
  failed?: number
  /** 読めなかったファイルのパス。100 件まで */
  failedPaths?: readonly string[]
}

export interface ScanState {
  scanning: boolean
  /** スキャン中は見つけた音声の数 */
  count: number
  last: ScanReport | null
}

/** 最後のスキャンの結果の文。 */
export function scanReportMessage(report: ScanReport, now: number): string {
  const when = formatAgo(report.at, now)
  const kind = report.full ? '全ファイルの読み直し' : 'スキャン'
  if (report.error) {
    return `${when}の${kind}は失敗しました（${report.error}）`
  }
  const files = (report.files ?? 0).toLocaleString('ja-JP')
  return `${when}の${kind}: ${files} ファイル、読み直し ${report.read ?? 0} 件`
}
