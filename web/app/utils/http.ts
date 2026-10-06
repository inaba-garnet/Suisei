/** `$fetch` が投げた失敗の、応答の状態コード。応答がなければ `undefined`。 */
export function httpStatusOf(err: unknown): number | undefined {
  return (err as { response?: { status?: number } }).response?.status
}
