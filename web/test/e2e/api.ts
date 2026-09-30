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

export interface MockAlbum {
  id: string
  name: string
  artist: string
  songCount: number
  duration: number
}

/** `/rest` のライブラリの偽物。`count` 枚のアルバムを名前順で返し、呼ばれた引数を `requests` に残す。 */
export async function mockLibrary(page: Page, { count = 250 } = {}) {
  const albums: MockAlbum[] = Array.from({ length: count }, (_, i) => ({
    id: `al-${i}`,
    name: `アルバム ${String(i).padStart(3, '0')}`,
    artist: `アーティスト ${i % 7}`,
    songCount: 2,
    duration: 400,
  }))
  const requests: URLSearchParams[] = []
  const ok = (body: object) => ({ json: { 'subsonic-response': { status: 'ok', version: '1.16.1', ...body } } })

  await page.route(/\/rest\/getAlbumList2(\?|$)/, (route) => {
    const params = new URL(route.request().url()).searchParams
    requests.push(params)
    const offset = Number(params.get('offset') ?? 0)
    const size = Number(params.get('size') ?? 10)
    return route.fulfill(ok({ albumList2: { album: albums.slice(offset, offset + size) } }))
  })
  await page.route(/\/rest\/getAlbum(\?|$)/, (route) => {
    const id = new URL(route.request().url()).searchParams.get('id')
    const album = albums.find(a => a.id === id)
    if (!album) {
      return route.fulfill(ok({}))
    }
    return route.fulfill(ok({
      album: {
        ...album,
        year: 2024,
        genres: [{ name: 'Pop' }],
        song: [
          { id: `${id}-1`, title: '一曲目', artist: album.artist, track: 1, discNumber: 1, duration: 200 },
          { id: `${id}-2`, title: '二曲目', artist: 'ゲスト', track: 2, discNumber: 1, duration: 200 },
        ],
      },
    }))
  })
  await page.route(/\/rest\/getCoverArt(\?|$)/, route => route.fulfill({ status: 404 }))
  return { albums, requests }
}
