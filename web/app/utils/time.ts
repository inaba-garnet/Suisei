/** 経過時間を「たった今」「5 分前」「3 時間前」「2 日前」の形にする。 */
export function formatAgo(at: number, now: number): string {
  const minutes = Math.floor(Math.max(0, now - at) / 60_000)
  if (minutes < 1) {
    return 'たった今'
  }
  if (minutes < 60) {
    return `${minutes} 分前`
  }
  const hours = Math.floor(minutes / 60)
  if (hours < 24) {
    return `${hours} 時間前`
  }
  return `${Math.floor(hours / 24)} 日前`
}
