/** ログイン中の利用者。`undefined` はまだ確かめていない、`null` はログインしていない。 */
export function useUser() {
  return useState<string | null | undefined>('user', () => undefined)
}

export class LoginError extends Error {}

export function useAuth() {
  const user = useUser()
  const serverDev = useServerDev()

  /** `GET /api/me` でログイン中かを確かめる（docs/server.md の「Web クライアントのログイン」）。 */
  async function fetchMe(): Promise<string | null> {
    try {
      const me = await $fetch<{ username: string, dev?: boolean }>('/api/me')
      user.value = me.username
      serverDev.value = me.dev === true
    }
    catch (err) {
      if (statusOf(err) !== 401) {
        throw err
      }
      user.value = null
      serverDev.value = false
    }
    return user.value
  }

  async function login(username: string, password: string): Promise<void> {
    try {
      await $fetch('/api/login', { method: 'POST', body: { username, password } })
    }
    catch (err) {
      if (statusOf(err) === 401) {
        throw new LoginError('ユーザー名かパスワードが違います')
      }
      // 失敗が続くと、サーバーはしばらくログインを拒む（docs/server.md の「認証の総当たりの制限」）
      if (statusOf(err) === 429) {
        throw new LoginError('失敗が続いたため、しばらく待ってからやり直してください')
      }
      throw err
    }
    // 開発モードかどうかも受け取るため、ログイン中の利用者を取り直す
    await fetchMe()
  }

  async function logout(): Promise<void> {
    // $fetch は body がないと Content-Type を付けないので、空の JSON を送る
    await $fetch('/api/logout', { method: 'POST', body: {} })
    user.value = null
    serverDev.value = false
  }

  return { user, fetchMe, login, logout }
}

function statusOf(err: unknown): number | undefined {
  return (err as { response?: { status?: number } }).response?.status
}
