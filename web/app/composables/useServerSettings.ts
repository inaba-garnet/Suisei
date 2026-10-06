import type { ServerSettings } from '~/utils/settings'

/**
 * サーバーの設定（docs/web.md の「設定」）。画面をまたいで同じ値を使う。
 * `save` は変える項目だけを受け、ほかは今の値のまま全体を送る。失敗したら元の値に戻して投げ直す。
 * `update` は `save` の結果を知らせに出す。選んだらすぐ保存する欄に使う。
 */
export function useServerSettings() {
  const settings = useState<ServerSettings | undefined>('server-settings', () => undefined)
  const saving = useState('server-settings-saving', () => false)
  const toast = useToast()

  async function load(): Promise<void> {
    settings.value = await $fetch<ServerSettings>('/api/settings')
  }

  async function save(patch: Partial<ServerSettings>): Promise<void> {
    const previous = settings.value
    if (!previous) {
      return
    }
    settings.value = { ...previous, ...patch }
    try {
      settings.value = await $fetch<ServerSettings>('/api/settings', { method: 'PUT', body: settings.value })
    }
    catch (err) {
      settings.value = previous
      throw err
    }
  }

  async function update(patch: Partial<ServerSettings>): Promise<void> {
    saving.value = true
    try {
      await save(patch)
      toast.show('保存しました')
    }
    catch {
      toast.show('保存できませんでした')
    }
    finally {
      saving.value = false
    }
  }

  return { settings, saving: readonly(saving), load, save, update }
}
