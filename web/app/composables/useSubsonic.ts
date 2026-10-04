import { API_VERSION, CLIENT, SubsonicError, unwrap } from '~/utils/subsonic'

export type SubsonicParams = Record<string, string | number | (string | number)[]>

/**
 * Subsonic API を JSON で呼ぶ関数を返す。エラー 40 ならログインの画面に移す（docs/web.md の「一覧」）。
 * 配列の引数は同じ名前で繰り返して送る。`post` なら引数をフォームで送る（`formPost` 拡張）。プレイリストの曲のように、URL に収まらない数の引数を送るため。
 */
export function useSubsonic() {
  const user = useUser()
  const route = useRoute()

  return async function call<T>(endpoint: string, params: SubsonicParams = {}, { post = false } = {}): Promise<T> {
    const all: SubsonicParams = { ...params, f: 'json', c: CLIENT, v: API_VERSION }
    try {
      const body = post
        ? await $fetch(`/rest/${endpoint}`, { method: 'POST', body: formOf(all) })
        : await $fetch(`/rest/${endpoint}`, { query: all })
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

function formOf(params: SubsonicParams): URLSearchParams {
  const form = new URLSearchParams()
  for (const [key, value] of Object.entries(params)) {
    for (const v of Array.isArray(value) ? value : [value]) {
      form.append(key, String(v))
    }
  }
  return form
}
