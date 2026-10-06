/** サーバーの設定（`/api/settings`、docs/server.md の「設定」）の型と、選ぶ候補（docs/web.md の「設定」）。 */

export interface ServerSettings {
  /** 定期スキャンの間隔（秒）。0 で止める */
  scanInterval: number
  /** 定期のバックアップの間隔（秒）。0 で止める */
  backupInterval: number
  /** 残す定期のバックアップの数 */
  backupKeep: number
  /** `キャラクター(CV:声優)` を分けるか */
  splitCharacters: boolean
  spotifyClientId: string | null
}

const MINUTE = 60
const HOUR = 60 * MINUTE
const DAY = 24 * HOUR

export const scanIntervals = [15 * MINUTE, 30 * MINUTE, HOUR, 3 * HOUR, 6 * HOUR, 12 * HOUR, DAY, 0]
export const backupIntervals = [6 * HOUR, 12 * HOUR, DAY, 3 * DAY, 7 * DAY, 0]
export const backupKeeps = [1, 3, 5, 7, 14, 30]

/** 間隔を「15 分」「1 時間」「1 日」の形にする。0 は「止める」。割り切れない値は細かい単位で出す。 */
export function intervalLabel(seconds: number): string {
  if (seconds === 0) {
    return '止める'
  }
  if (seconds % DAY === 0) {
    return `${seconds / DAY} 日`
  }
  if (seconds % HOUR === 0) {
    return `${seconds / HOUR} 時間`
  }
  if (seconds % MINUTE === 0) {
    return `${seconds / MINUTE} 分`
  }
  return `${seconds} 秒`
}

/** 候補に今の値がなければ足す。「止める」は末尾に残す。 */
export function withCurrent(candidates: number[], current: number): number[] {
  if (candidates.includes(current)) {
    return candidates
  }
  const values = [...candidates.filter(v => v !== 0), current].sort((a, b) => a - b)
  return candidates.includes(0) ? [...values, 0] : values
}
