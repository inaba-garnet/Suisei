import type { Album, Song } from '~/utils/subsonic'

/** メニューで操作する対象。アルバムは、操作をアルバムの全曲に対して行う。 */
export type SongMenuTarget = { kind: 'song', song: Song } | { kind: 'album', album: Album, songs: Song[] }

interface SongMenuState {
  target: SongMenuTarget
  /** 開いた「…」のボタン。PC はその横に出し、閉じたらフォーカスを戻す */
  anchor: HTMLElement
}

/**
 * 曲とアルバムの「…」のメニュー（docs/web.md の「メニュー」）。メニューは layout に一つだけ置き、行の「…」は開く対象とボタンを渡す。
 * 曲の一覧は見える行だけを描いて行を作り直すので、行ごとにメニューを持たせないため。
 */
export function useSongMenu() {
  const state = useState<SongMenuState | undefined>('song-menu', () => undefined)

  function open(target: SongMenuTarget, event: Event) {
    state.value = { target, anchor: event.currentTarget as HTMLElement }
  }

  function close() {
    const anchor = state.value?.anchor
    state.value = undefined
    anchor?.focus({ preventScroll: true })
  }

  return { state: computed(() => state.value), open, close }
}

/** 画面の下に短く出す知らせ。新しい知らせが来たら置き換える。 */
export function useToast() {
  const message = useState<{ text: string, id: number } | undefined>('toast', () => undefined)
  let timer: ReturnType<typeof setTimeout> | undefined

  function show(text: string) {
    const id = (message.value?.id ?? 0) + 1
    message.value = { text, id }
    clearTimeout(timer)
    timer = setTimeout(() => {
      if (message.value?.id === id) {
        message.value = undefined
      }
    }, 3000)
  }

  return { message: readonly(message), show }
}
