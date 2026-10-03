/** スマホで下から引き出す再生画面を開いているか（docs/web.md の「再生」）。 */
export function usePlayerSheet() {
  return useState('player-sheet', () => false)
}

/** キューを開いているか（docs/web.md の「再生」）。スマホは再生画面に重ね、PC は右端か右の欄に出す。 */
export function usePlayerQueueOpen() {
  return useState('player-queue', () => false)
}
