import type { Page } from '@playwright/test'

export const USER = 'inaba'
export const PASSWORD = 'sesame'

/** `/api` の偽物。ログインの状態をテストの中に持つ。 */
export async function mockApi(page: Page, { loggedIn = false, dev = false } = {}) {
  let session = loggedIn
  await page.route('**/api/me', route =>
    session
      ? route.fulfill({ json: { username: USER, dev } })
      : route.fulfill({ status: 401 }))
  await page.route('**/api/login', async (route) => {
    const request = route.request()
    const body = request.postDataJSON() as { username: string, password: string }
    const json = request.headers()['content-type']?.startsWith('application/json')
    if (!json) {
      return route.fulfill({ status: 415 })
    }
    if (body.username !== USER || body.password !== PASSWORD) {
      return route.fulfill({ status: 401 })
    }
    session = true
    return route.fulfill({ status: 204 })
  })
  await page.route('**/api/logout', async (route) => {
    const json = route.request().headers()['content-type']?.startsWith('application/json')
    if (!json) {
      return route.fulfill({ status: 415 })
    }
    session = false
    return route.fulfill({ status: 204 })
  })
}
