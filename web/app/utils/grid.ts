/**
 * ジャケットのグリッドの列の数（docs/web.md の「一覧」）。内容の幅に合わせて 2〜8 列にし、
 * 広い画面でジャケットが大きくなりすぎないようにする。
 */
export function gridColumns(width: number): number {
  const steps: [number, number][] = [[1280, 8], [1120, 7], [960, 6], [800, 5], [480, 4]]
  return steps.find(([min]) => width >= min)?.[1] ?? 2
}
