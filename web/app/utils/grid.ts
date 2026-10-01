/**
 * ジャケットのグリッドの列の数（docs/web.md の「一覧」）。内容の幅に合わせて 2〜6 列にし、
 * 広い画面でジャケットが大きくなりすぎないようにする。
 */
export function gridColumns(width: number): number {
  if (width >= 960) {
    return 6
  }
  if (width >= 800) {
    return 5
  }
  return width >= 480 ? 4 : 2
}
