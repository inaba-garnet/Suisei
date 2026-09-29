/** ログイン中の利用者。`undefined` はまだ確かめていない、`null` はログインしていない。 */
export function useUser() {
  return useState<string | null | undefined>('user', () => undefined)
}

export class LoginError extends Error {}

export function useAuth() {
  const user = useUser()

  /** `GET /api/me` でログイン中かを確かめる（docs/server.md の「Web クライアントのログイン」）。 */
  async function fetchMe(): Promise<string | null> {
    try {
      const me = await $fetch<{ username: string }>('/api/me')
      user.value = me.username
    }
    catch (err) {
      if (statusOf(err) !== 401) {
        throw err
      }
      user.value = null
    }
    return user.value
  }

  async function login(username: string, password: string): Promise<void> {
    try {
      await $fetch('/api/login', { method: 'POST', body: { username, password } })
    }
    catch (err) {
      if (statusOf(err) === 401) {
        throw new LoginError('利用者名かパスワードが違います')
      }
      throw err
    }
    user.value = username
  }

  async function logout(): Promise<void> {
    // $fetch は body がないと Content-Type を付けないので、空の JSON を送る
    await $fetch('/api/logout', { method: 'POST', body: {} })
    user.value = null
  }

  return { user, fetchMe, login, logout }
}

function statusOf(err: unknown): number | undefined {
  return (err as { response?: { status?: number } }).response?.status
}
