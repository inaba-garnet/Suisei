/** 読みを直す対象。曲、アルバム、アーティストの ID と、見出しに出す名前。 */
export interface ReadingTarget {
  id: string
  name: string
}

/**
 * 読みを直すモーダル（docs/web.md の「読みの編集」）。モーダルは layout に一つだけ置き、メニューは対象を渡して開く。
 * 曲の行ごとにモーダルを持たせないため。
 */
export function useReadingEditor() {
  const target = useState<ReadingTarget | undefined>('reading-editor', () => undefined)

  function open(next: ReadingTarget) {
    target.value = next
  }

  function close() {
    target.value = undefined
  }

  return { target: computed(() => target.value), open, close }
}
