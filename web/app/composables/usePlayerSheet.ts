/** スマホで下から引き出す再生画面を開いているか（docs/web.md の「再生」）。 */
export function usePlayerSheet() {
  return useState('player-sheet', () => false)
}
