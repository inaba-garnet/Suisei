/** サーバーが `GET /api/me` で返す開発モードのフラグ。 */
export function useServerDev() {
  return useState<boolean>('server-dev', () => false)
}

/** 開発モードか（docs/web.md の「開発」）。サーバーの `SUISEI_DEV` か、Nuxt の開発サーバーで動かしているとき。 */
export function useDev() {
  const serverDev = useServerDev()
  return computed(() => import.meta.dev || serverDev.value)
}
