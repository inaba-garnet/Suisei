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
export async function mockLibrary(page: Page, { count = 250, songs = 2, name = (i: number) => `アルバム ${String(i).padStart(3, '0')}` } = {}) {
  const albums: MockAlbum[] = Array.from({ length: count }, (_, i) => ({
    id: `al-${i}`,
    name: name(i),
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
          ...Array.from({ length: Math.max(0, songs - 2) }, (_, i) => (
            { id: `${id}-${i + 3}`, title: `曲 ${i + 3}`, artist: album.artist, track: i + 3, discNumber: 1, duration: 200 }
          )),
        ],
      },
    }))
  })
  await page.route(/\/rest\/getCoverArt(\?|$)/, route => route.fulfill({ status: 404 }))
  return { albums, requests }
}

/** `/rest` のアーティストの偽物。読みの行ごとに `perRow` 人ずつ並べ、呼ばれた回数を `calls` に数える。 */
export async function mockArtists(page: Page, { rows = ['ア', 'カ', 'サ', 'タ', 'A', 'B'], perRow = 12 } = {}) {
  const index = rows.map(row => ({
    name: row,
    artist: Array.from({ length: perRow }, (_, i) => ({ id: `ar-${row}-${i}`, name: `${row}のアーティスト ${i}`, albumCount: 2 })),
  }))
  const calls = { artists: 0 }
  const ok = (body: object) => ({ json: { 'subsonic-response': { status: 'ok', version: '1.16.1', ...body } } })
  await page.route(/\/rest\/getArtists(\?|$)/, (route) => {
    calls.artists++
    return route.fulfill(ok({ artists: { ignoredArticles: '', index } }))
  })
  await page.route(/\/rest\/getArtist(\?|$)/, (route) => {
    const id = new URL(route.request().url()).searchParams.get('id')!
    const artist = index.flatMap(group => group.artist).find(a => a.id === id)
    return route.fulfill(ok({
      artist: {
        ...artist,
        album: [
          { id: `${id}-al-1`, name: `${artist?.name} の一枚目`, artist: artist?.name, songCount: 10, duration: 2400 },
          { id: `${id}-al-2`, name: `${artist?.name} の二枚目`, artist: artist?.name, songCount: 12, duration: 2800 },
        ],
      },
    }))
  })
  await page.route(/\/rest\/getCoverArt(\?|$)/, route => route.fulfill({ status: 404 }))
  return { index, calls }
}
