import { API_VERSION, CLIENT, SubsonicError, unwrap } from '~/utils/subsonic'

/** Subsonic API を JSON で呼ぶ関数を返す。エラー 40 ならログインの画面に移す（docs/web.md の「一覧」）。 */
export function useSubsonic() {
  const user = useUser()
  const route = useRoute()

  return async function call<T>(endpoint: string, params: Record<string, string | number> = {}): Promise<T> {
    try {
      const body = await $fetch(`/rest/${endpoint}`, {
        query: { ...params, f: 'json', c: CLIENT, v: API_VERSION },
      })
      return unwrap<T>(body)
    }
    catch (err) {
      if (err instanceof SubsonicError && err.code === 40) {
        user.value = null
        await navigateTo({ path: '/login', query: { redirect: route.fullPath } })
      }
      throw err
    }
  }
}
