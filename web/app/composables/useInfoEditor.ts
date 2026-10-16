/** 情報を直す対象。曲、アルバム、アーティストの ID と、見出しに出す名前。 */
export interface InfoTarget {
  id: string
  name: string
}

/**
 * 曲、アルバム、アーティストの情報を直すモーダル（docs/web.md の「情報の編集」）。モーダルは layout に一つだけ置き、メニューは対象を渡して開く。
 * 曲の行ごとにモーダルを持たせないため。
 */
export function useInfoEditor() {
  const target = useState<InfoTarget | undefined>('info-editor', () => undefined)

  function open(next: InfoTarget) {
    target.value = next
  }

  function close() {
    target.value = undefined
  }

  return { target: computed(() => target.value), open, close }
}
