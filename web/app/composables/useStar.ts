/** お気に入りにする対象。アルバムは Subsonic の `albumId`、曲は `id` で渡す。 */
export type StarTarget = 'album' | 'song'

/**
 * お気に入りの付け外し（docs/web.md の「一覧」）。`starred` はすぐに切り替え、サーバーが失敗を返したら元に戻す。
 * 戻り値の `toggle` は、付け外しの後の状態を返す。
 */
export function useStar(target: StarTarget, id: MaybeRefOrGetter<string>, initial: MaybeRefOrGetter<boolean>) {
  const subsonic = useSubsonic()
  const starred = ref(toValue(initial))
  watch(() => toValue(initial), (value) => {
    starred.value = value
  })
  const pending = ref(false)

  async function toggle() {
    if (pending.value) {
      return starred.value
    }
    const next = !starred.value
    starred.value = next
    pending.value = true
    try {
      await subsonic(next ? 'star' : 'unstar', { [target === 'album' ? 'albumId' : 'id']: toValue(id) })
    }
    catch {
      starred.value = !next
    }
    finally {
      pending.value = false
    }
    return starred.value
  }

  return { starred, toggle }
}
