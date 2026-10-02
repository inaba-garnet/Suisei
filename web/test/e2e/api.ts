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
    // お気に入りは 3 枚ごとに 1 枚
    const list = params.get('type') === 'starred' ? albums.filter((_, i) => i % 3 === 0) : albums
    return route.fulfill(ok({ albumList2: { album: list.slice(offset, offset + size) } }))
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
        coverArt: album.id,
        artists: [{ id: 'ar-1', name: album.artist }],
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

/**
 * `/rest` のアーティストの偽物。読みの行ごとに `perRow` 人ずつ並べ、呼ばれた回数を `calls` に数える。
 * `getArtists` の `role` がアルバムアーティスト以外なら、名前の前に役割を付けて別の人にする。受けた `role` は `roles` に残す。
 */
export async function mockArtists(page: Page, { rows = ['ア', 'カ', 'サ', 'タ', 'A', 'B'], perRow = 12 } = {}) {
  const index = rows.map(row => ({
    name: row,
    artist: Array.from({ length: perRow }, (_, i) => ({ id: `ar-${row}-${i}`, name: `${row}のアーティスト ${i}`, albumCount: 2, ...(row === 'カ' && i < 2 && { starred: '2026-01-01T00:00:00Z' }) })),
  }))
  const calls = { artists: 0 }
  const roles: string[] = []
  /** `getArtist` に渡った `songs`。渡らなければ空文字 */
  const songs: string[] = []
  const ok = (body: object) => ({ json: { 'subsonic-response': { status: 'ok', version: '1.16.1', ...body } } })
  await page.route(/\/rest\/getArtists(\?|$)/, (route) => {
    calls.artists++
    const role = new URL(route.request().url()).searchParams.get('role') ?? 'albumartist'
    roles.push(role)
    const named = role === 'albumartist'
      ? index
      : index.map(group => ({ ...group, artist: group.artist.map(a => ({ ...a, name: `${role}:${a.name}` })) }))
    return route.fulfill(ok({ artists: { ignoredArticles: '', index: named } }))
  })
  await page.route(/\/rest\/getArtist(\?|$)/, (route) => {
    const params = new URL(route.request().url()).searchParams
    const id = params.get('id')!
    const artist = index.flatMap(group => group.artist).find(a => a.id === id)
    const self = [{ id, name: artist?.name ?? '' }]
    const guest = { id: `${id}-al-g`, name: '客演のアルバム', artist: 'ほかの人', artists: [{ id: 'ar-other', name: 'ほかの人' }], year: 2015, songCount: 8, duration: 2000 }
    songs.push(params.get('songs') ?? '')
    return route.fulfill(ok({
      artist: {
        ...artist,
        album: [
          { id: `${id}-al-1`, name: `${artist?.name} の一枚目`, artist: artist?.name, artists: self, year: 2010, songCount: 10, duration: 2400 },
          { id: `${id}-al-2`, name: `${artist?.name} の二枚目`, artist: artist?.name, artists: self, year: 2020, songCount: 12, duration: 2800 },
          guest,
        ],
        ...(params.get('songs') && {
          song: [
            { id: 's-1', title: '一曲目', artist: artist?.name, album: `${artist?.name} の一枚目`, albumId: `${id}-al-1`, year: 2010, track: 1, duration: 200 },
            { id: 's-2', title: '二曲目', artist: artist?.name, album: `${artist?.name} の一枚目`, albumId: `${id}-al-1`, year: 2010, track: 2, duration: 210 },
            { id: 's-3', title: '客演の曲', artist: `ほかの人 feat. ${artist?.name}`, album: guest.name, albumId: guest.id, year: 2015, track: 5, duration: 220 },
          ],
        }),
      },
    }))
  })
  await page.route(/\/rest\/getCoverArt(\?|$)/, route => route.fulfill({ status: 404 }))
  return { index, calls, roles, songs }
}

/** `search3` で全曲を返す偽物。並び順（`songSort`）を曲名の頭に付け、呼ばれた引数を `requests` に残す。 */
export async function mockTracks(page: Page, { count = 250 } = {}) {
  const requests: URLSearchParams[] = []
  const ok = (body: object) => ({ json: { 'subsonic-response': { status: 'ok', version: '1.16.1', ...body } } })
  await page.route(/\/rest\/search3(\?|$)/, (route) => {
    const params = new URL(route.request().url()).searchParams
    requests.push(params)
    const sort = params.get('songSort') ?? 'title'
    const offset = Number(params.get('songOffset') ?? 0)
    const size = Number(params.get('songCount') ?? 20)
    const song = Array.from({ length: Math.max(0, Math.min(size, count - offset)) }, (_, i) => {
      const n = String(offset + i).padStart(3, '0')
      return { id: `tr-${n}`, title: `${sort}:曲 ${n}`, artist: `アーティスト ${n}`, album: `アルバム ${n}`, albumId: `al-${n}`, duration: 200 }
    })
    return route.fulfill(ok({ searchResult3: song.length ? { song } : {} }))
  })
  await page.route(/\/rest\/getCoverArt(\?|$)/, route => route.fulfill({ status: 404 }))
  return { requests }
}

/** ジャンルと、ジャンルごとの曲の偽物。呼ばれた `getSongsByGenre` の引数を `requests` に残す。 */
export async function mockGenres(page: Page) {
  const genres = [
    { value: 'ロック', songCount: 3, albumCount: 1 },
    { value: 'アニソン', songCount: 12, albumCount: 2 },
    { value: 'J-Pop', songCount: 3, albumCount: 1 },
  ]
  const requests: URLSearchParams[] = []
  const ok = (body: object) => ({ json: { 'subsonic-response': { status: 'ok', version: '1.16.1', ...body } } })
  await page.route(/\/rest\/getGenres(\?|$)/, route => route.fulfill(ok({ genres: { genre: genres } })))
  await page.route(/\/rest\/getSongsByGenre(\?|$)/, (route) => {
    const params = new URL(route.request().url()).searchParams
    requests.push(params)
    const genre = params.get('genre') ?? ''
    const sort = params.get('songSort') ?? 'title'
    const song = Number(params.get('offset') ?? 0) > 0
      ? []
      : [1, 2, 3].map(i => ({ id: `${genre}-${i}`, title: `${sort}:${genre} の曲 ${i}`, artist: 'だれか', album: `${genre} のアルバム`, duration: 200 }))
    return route.fulfill(ok({ songsByGenre: { song } }))
  })
  await page.route(/\/rest\/getCoverArt(\?|$)/, route => route.fulfill({ status: 404 }))
  return { genres, requests }
}

/** お気に入りの曲の偽物。`songSort` があれば曲名の頭に付け、呼ばれた引数を `requests` に残す。 */
export async function mockFavorites(page: Page) {
  const requests: URLSearchParams[] = []
  const ok = (body: object) => ({ json: { 'subsonic-response': { status: 'ok', version: '1.16.1', ...body } } })
  await page.route(/\/rest\/getStarred2(\?|$)/, (route) => {
    const params = new URL(route.request().url()).searchParams
    requests.push(params)
    const sort = params.get('songSort') ?? 'starred'
    const song = [1, 2, 3].map(i => ({ id: `fav-${i}`, title: `${sort}:お気に入り ${i}`, artist: 'だれか', album: 'どこか', duration: 200 }))
    return route.fulfill(ok({ starred2: { song } }))
  })
  await page.route(/\/rest\/getCoverArt(\?|$)/, route => route.fulfill({ status: 404 }))
  return { requests }
}

/** お気に入りの付け外しの偽物。呼ばれた引数を `calls` に残し、`fail` のときは失敗を返す。 */
export async function mockStar(page: Page, { fail = false } = {}) {
  const calls: { endpoint: string, params: URLSearchParams }[] = []
  await page.route(/\/rest\/(star|unstar)(\?|$)/, (route) => {
    const url = new URL(route.request().url())
    calls.push({ endpoint: url.pathname.split('/').at(-1)!, params: url.searchParams })
    const body = fail
      ? { status: 'failed', version: '1.16.1', error: { code: 0, message: 'failed' } }
      : { status: 'ok', version: '1.16.1' }
    return route.fulfill({ json: { 'subsonic-response': body } })
  })
  return { calls }
}
