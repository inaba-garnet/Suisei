import { useIntervalFn } from '@vueuse/core'

interface ScanStatus {
  scanning: boolean
  count: number
}

/**
 * 手動のスキャン（docs/web.md の「設定」）。画面をまたいで同じ状態を使う。
 * 走っている間は `getScanStatus` を 2 秒ごとに呼び、終わったら知らせを出す。
 */
export function useLibraryScan() {
  const subsonic = useSubsonic()
  const toast = useToast()
  const scanning = useState('library-scanning', () => false)
  const count = useState('library-scan-count', () => 0)

  const polling = useIntervalFn(refresh, 2000, { immediate: false })

  function apply(status: ScanStatus) {
    const finished = scanning.value && !status.scanning
    scanning.value = status.scanning
    count.value = status.count
    if (status.scanning) {
      polling.resume()
    }
    else {
      polling.pause()
    }
    if (finished) {
      toast.show('スキャンが終わりました')
    }
  }

  async function refresh() {
    try {
      apply((await subsonic<{ scanStatus: ScanStatus }>('getScanStatus')).scanStatus)
    }
    catch {
      // 状態を読めなくても、次の呼び出しでやり直す
    }
  }

  /** スキャンを始める。`full` なら全ファイルを読み直す。走っていれば新しく始めず、その進みを出す。 */
  async function start(full = false) {
    try {
      const res = await subsonic<{ scanStatus: ScanStatus }>('startScan', full ? { fullScan: 'true' } : {})
      apply(res.scanStatus)
    }
    catch {
      toast.show('スキャンを始められませんでした')
    }
  }

  return { scanning: readonly(scanning), count: readonly(count), refresh, start }
}
