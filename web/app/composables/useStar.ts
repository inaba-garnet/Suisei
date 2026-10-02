/** お気に入りにする対象。アルバムは Subsonic の `albumId`、曲は `id` で渡す。 */
export type StarTarget = 'album' | 'song'

/**
 * お気に入りの付け外し（docs/web.md の「一覧」）。`starred` はすぐに切り替え、サーバーが失敗を返したら元に戻す。
 * 付け外しの結果はページを読み直すまで対象と ID ごとに覚え、読み込んだときの `initial` より先に見る。
 * 戻り値の `toggle` は、付け外しの後の状態を返す。
 */
export function useStar(target: StarTarget, id: MaybeRefOrGetter<string>, initial: MaybeRefOrGetter<boolean>) {
  const subsonic = useSubsonic()
  const overrides = useState<Record<string, boolean>>('starred', () => ({}))
  const key = computed(() => `${target}:${toValue(id)}`)
  const starred = computed(() => overrides.value[key.value] ?? toValue(initial))
  const pending = ref(false)

  async function toggle() {
    if (pending.value) {
      return starred.value
    }
    const k = key.value
    const next = !starred.value
    overrides.value[k] = next
    pending.value = true
    try {
      await subsonic(next ? 'star' : 'unstar', { [target === 'album' ? 'albumId' : 'id']: toValue(id) })
    }
    catch {
      overrides.value[k] = !next
    }
    finally {
      pending.value = false
    }
    return starred.value
  }

  return { starred, toggle }
}
