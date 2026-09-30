/** ジャケットのグリッドの列の数。狭い画面（スマホ）は 2 列、広い画面は 4 列（docs/web.md の「一覧」）。 */
export function gridColumns(width: number): number {
  return width >= 480 ? 4 : 2
}
