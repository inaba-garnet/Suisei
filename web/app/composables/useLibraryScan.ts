import type { ScanReport, ScanState } from '~/utils/scan'
import { useIntervalFn } from '@vueuse/core'

/**
 * 手動のスキャンと、スキャンの状態（docs/web.md の「設定」）。画面をまたいで同じ状態を使う。
 * 走っている間は `GET /api/scan` を 2 秒ごとに呼び、終わったら知らせを出す。
 */
export function useLibraryScan() {
  const subsonic = useSubsonic()
  const toast = useToast()
  const scanning = useState('library-scanning', () => false)
  const count = useState('library-scan-count', () => 0)
  const last = useState<ScanReport | null>('library-scan-last', () => null)

  const polling = useIntervalFn(refresh, 2000, { immediate: false })

  function apply(state: Pick<ScanState, 'scanning' | 'count'> & { last?: ScanReport | null }) {
    const finished = scanning.value && !state.scanning
    scanning.value = state.scanning
    count.value = state.count
    if (state.last !== undefined) {
      last.value = state.last
    }
    if (state.scanning) {
      polling.resume()
    }
    else {
      polling.pause()
    }
    if (finished) {
      toast.show(last.value?.error ? 'スキャンに失敗しました' : 'スキャンが終わりました')
    }
  }

  async function refresh() {
    try {
      apply(await $fetch<ScanState>('/api/scan'))
    }
    catch {
      // 状態を読めなくても、次の呼び出しでやり直す
    }
  }

  /** スキャンを始める。`full` なら全ファイルを読み直す。走っていれば新しく始めず、その進みを出す。 */
  async function start(full = false) {
    try {
      const res = await subsonic<{ scanStatus: { scanning: boolean, count: number } }>('startScan', full ? { fullScan: 'true' } : {})
      apply(res.scanStatus)
    }
    catch {
      toast.show('スキャンを始められませんでした')
    }
  }

  return { scanning: readonly(scanning), count: readonly(count), last: readonly(last), refresh, start }
}
