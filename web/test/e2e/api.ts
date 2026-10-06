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
  year: number
  songCount: number
  duration: number
}

/** `/rest` のライブラリの偽物。`count` 枚のアルバムを名前順で返し、呼ばれた引数を `requests` に残す。 */
export async function mockLibrary(page: Page, { count = 250, songs = 2, name = (i: number) => `アルバム ${String(i).padStart(3, '0')}` } = {}) {
  const albums: MockAlbum[] = Array.from({ length: count }, (_, i) => ({
    id: `al-${i}`,
    name: name(i),
    artist: `アーティスト ${i % 7}`,
    year: 2000 + (i % 20),
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
    // お気に入りは 3 枚ごとに 1 枚。並べ方は区別せず、どれも名前順で返す
    const starred = params.get('type') === 'starred' || params.get('starred') === 'true'
    const list = starred ? albums.filter((_, i) => i % 3 === 0) : albums
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
        ].map(song => ({ ...song, album: album.name, albumId: album.id, coverArt: album.id, artists: [{ id: song.artist === 'ゲスト' ? 'ar-guest' : 'ar-1', name: song.artist }] })),
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
    const song = [1, 2, 3].map(i => ({ id: `fav-${i}`, title: `${sort}:お気に入り ${i}`, artist: 'だれか', album: 'どこか', duration: 200, starred: '2026-01-01T00:00:00Z' }))
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

/** 無音の WAV（8kHz、8 bit、モノラル）を `seconds` 秒ぶん作る。 */
function silence(seconds: number): Buffer {
  const samples = Math.round(8000 * seconds)
  const wav = Buffer.alloc(44 + samples, 128)
  wav.write('RIFF', 0)
  wav.writeUInt32LE(36 + samples, 4)
  wav.write('WAVEfmt ', 8)
  wav.writeUInt32LE(16, 16)
  wav.writeUInt16LE(1, 20)
  wav.writeUInt16LE(1, 22)
  wav.writeUInt32LE(8000, 24)
  wav.writeUInt32LE(8000, 28)
  wav.writeUInt16LE(1, 32)
  wav.writeUInt16LE(8, 34)
  wav.write('data', 36)
  wav.writeUInt32LE(samples, 40)
  return wav
}

/**
 * `stream` の偽物。どの曲にも `seconds` 秒の無音を返し、頼まれた曲の ID を `ids` に残す。`fail` なら 404 を返す。
 * シークで途中から読み直す要求と、ブラウザが同じ曲を続けて頭から読み直す要求は、同じ曲なので `ids` に残さない。
 * `unplayable` なら、元のファイルにはブラウザが鳴らせない中身を返し、`format` を付けた変換の要求にだけ無音を返す。
 * 変換の要求を含め、曲、形式、頭出しの位置が変わった要求を `requests` に残す。
 */
export async function mockStream(page: Page, { seconds = 30, fail = false, unplayable = false } = {}) {
  const ids: string[] = []
  const requests: { id: string, format: string | null, timeOffset: string | null }[] = []
  const body = silence(seconds)
  await page.route(/\/rest\/stream(\?|$)/, (route) => {
    // サーバーと同じく Range に応じる。応じないとブラウザがシークできない
    const range = /bytes=(\d+)-(\d*)/.exec(route.request().headers().range ?? '')
    const params = new URL(route.request().url()).searchParams
    const id = params.get('id')!
    const request = { id, format: params.get('format'), timeOffset: params.get('timeOffset') }
    const last = requests.at(-1)
    if (!last || last.id !== request.id || last.format !== request.format || last.timeOffset !== request.timeOffset) {
      requests.push(request)
    }
    if ((!range || Number(range[1]) === 0) && ids.at(-1) !== id) {
      ids.push(id)
    }
    if (fail) {
      return route.fulfill({ status: 404 })
    }
    if (unplayable && !request.format) {
      return route.fulfill({ status: 200, contentType: 'audio/x-ape', body: Buffer.from('MAC not really audio') })
    }
    // 変換した音声は、サーバーと同じく Range に応じない
    if (!range || request.format) {
      return route.fulfill({ status: 200, contentType: 'audio/wav', headers: { 'Accept-Ranges': request.format ? 'none' : 'bytes' }, body })
    }
    const start = Number(range[1])
    const end = range[2] ? Math.min(Number(range[2]), body.length - 1) : body.length - 1
    return route.fulfill({
      status: 206,
      contentType: 'audio/wav',
      headers: { 'Accept-Ranges': 'bytes', 'Content-Range': `bytes ${start}-${end}/${body.length}` },
      body: body.subarray(start, end + 1),
    })
  })
  return { ids, requests }
}

/** `scrobble` の偽物。呼ばれた引数を `calls` に残す。 */
export async function mockScrobble(page: Page) {
  const calls: URLSearchParams[] = []
  await page.route(/\/rest\/scrobble(\?|$)/, (route) => {
    calls.push(new URL(route.request().url()).searchParams)
    return route.fulfill({ json: { 'subsonic-response': { status: 'ok', version: '1.16.1' } } })
  })
  return { calls }
}

/**
 * Media Session に渡したハンドラを `window.mediaSessionHandlers` に残す。
 * テストから OS のロック画面の操作の代わりに呼ぶため。
 */
export async function recordMediaSession(page: Page) {
  await page.addInitScript(() => {
    const handlers: Record<string, ((details: object) => void) | null> = {}
    ;(window as unknown as { mediaSessionHandlers: typeof handlers }).mediaSessionHandlers = handlers
    const original = navigator.mediaSession.setActionHandler.bind(navigator.mediaSession)
    navigator.mediaSession.setActionHandler = (action, handler) => {
      handlers[action] = handler as typeof handlers[string]
      original(action, handler)
    }
  })
}

const searchRoles = [
  { roles: ['albumartist', 'artist'] },
  { roles: ['albumartist'] },
  { roles: ['composer', 'lyricist'] },
  {},
]

/**
 * `search3` の偽物。名前に検索語を含むアーティスト、アルバム、曲を、種類ごとの件数と位置で返す。
 * 呼ばれた引数を `requests` に残す。
 */
export async function mockSearch(page: Page, { artists = 8, albums = 30, songs = 150 } = {}) {
  const library = {
    // 役割は 4 人ごとに、曲のアーティスト、アルバムアーティストだけ、作曲家だけ、役割なしを繰り返す
    artist: Array.from({ length: artists }, (_, i) => ({ id: `ar-${i}`, name: `ひかりのアーティスト ${i}`, albumCount: 1, ...searchRoles[i % 4] })),
    album: Array.from({ length: albums }, (_, i) => ({ id: `al-${i}`, name: `ひかりのアルバム ${i}`, artist: 'ClariS', songCount: 10, duration: 2400 })),
    song: Array.from({ length: songs }, (_, i) => ({ id: `tr-${i}`, title: `ひかりの曲 ${String(i).padStart(3, '0')}`, artist: 'ClariS', album: 'ひかりのアルバム 0', albumId: 'al-0', duration: 200 })),
  }
  const requests: URLSearchParams[] = []
  const ok = (body: object) => ({ json: { 'subsonic-response': { status: 'ok', version: '1.16.1', ...body } } })
  await page.route(/\/rest\/search3(\?|$)/, (route) => {
    const params = new URL(route.request().url()).searchParams
    requests.push(params)
    const query = params.get('query') ?? ''
    const result: Record<string, object[]> = {}
    for (const kind of ['artist', 'album', 'song'] as const) {
      const count = Number(params.get(`${kind}Count`) ?? 20)
      const offset = Number(params.get(`${kind}Offset`) ?? 0)
      const hits = library[kind].filter(item => ('name' in item ? item.name : item.title).includes(query)).slice(offset, offset + count)
      if (hits.length) {
        result[kind] = hits
      }
    }
    return route.fulfill(ok({ searchResult3: result }))
  })
  await page.route(/\/rest\/getCoverArt(\?|$)/, route => route.fulfill({ status: 404 }))
  return { requests }
}

/**
 * プレイリストの偽物。作成、名前の変更、曲の入れ替え、削除を手元の一覧に反映し、
 * 呼ばれた `updatePlaylist` の引数（フォームか URL）を `updates` に残す。
 */
export async function mockPlaylists(page: Page, { count = 2, songs = 4 } = {}) {
  const song = (i: number) => ({ id: `tr-${i}`, title: `曲 ${i}`, artist: 'ClariS', album: `アルバム ${i % 2}`, albumId: `al-${i % 2}`, coverArt: `al-${i % 2}`, duration: 200 })
  const playlists = Array.from({ length: count }, (_, i) => ({
    id: `pl-${i}`,
    name: `プレイリスト ${i}`,
    entry: Array.from({ length: songs }, (_, j) => song(j)),
  }))
  const updates: URLSearchParams[] = []
  const ok = (body: object) => ({ json: { 'subsonic-response': { status: 'ok', version: '1.16.1', ...body } } })
  const summary = (p: typeof playlists[number]) => ({
    id: p.id,
    name: p.name,
    songCount: p.entry.length,
    duration: p.entry.length * 200,
    changed: '2026-10-01T00:00:00Z',
    ...(p.entry[0] && { coverArt: p.entry[0].coverArt }),
  })
  const paramsOf = (request: { url: () => string, method: () => string, postData: () => string | null }) =>
    request.method() === 'POST' ? new URLSearchParams(request.postData() ?? '') : new URL(request.url()).searchParams

  await page.route(/\/rest\/getPlaylists(\?|$)/, route => route.fulfill(ok({ playlists: playlists.length ? { playlist: playlists.map(summary) } : {} })))
  await page.route(/\/rest\/getPlaylist(\?|$)/, (route) => {
    const p = playlists.find(p => p.id === paramsOf(route.request()).get('id'))
    return p
      ? route.fulfill(ok({ playlist: { ...summary(p), entry: p.entry } }))
      : route.fulfill(ok({ status: 'failed', error: { code: 70, message: 'playlist not found' } }))
  })
  await page.route(/\/rest\/createPlaylist(\?|$)/, (route) => {
    const p = { id: `pl-${playlists.length}`, name: paramsOf(route.request()).get('name')!, entry: [] as ReturnType<typeof song>[] }
    playlists.push(p)
    return route.fulfill(ok({ playlist: { ...summary(p), entry: [] } }))
  })
  await page.route(/\/rest\/updatePlaylist(\?|$)/, (route) => {
    const params = paramsOf(route.request())
    updates.push(params)
    const p = playlists.find(p => p.id === params.get('playlistId'))!
    if (params.get('name')) {
      p.name = params.get('name')!
    }
    const remove = new Set(params.getAll('songIndexToRemove').map(Number))
    const all = [...playlists.flatMap(p => p.entry), ...Array.from({ length: 10 }, (_, i) => song(i))]
    p.entry = [
      ...p.entry.filter((_, i) => !remove.has(i)),
      ...params.getAll('songIdToAdd').map(id => all.find(s => s.id === id) ?? { ...song(0), id, title: id }),
    ]
    return route.fulfill(ok({}))
  })
  await page.route(/\/rest\/deletePlaylist(\?|$)/, (route) => {
    const index = playlists.findIndex(p => p.id === paramsOf(route.request()).get('id'))
    playlists.splice(index, 1)
    return route.fulfill(ok({}))
  })
  await page.route(/\/rest\/getCoverArt(\?|$)/, route => route.fulfill({ status: 404 }))
  return { playlists, updates }
}

/**
 * `getLyricsBySongId` の偽物。`synced` の曲には 3 秒ごとの時刻付きの歌詞、`plain` の曲には時刻なしの歌詞を返し、ほかの曲には歌詞を返さない。
 * 頼まれた曲の ID を `ids` に残す。
 */
export async function mockLyrics(page: Page, { synced = [] as string[], plain = [] as string[], lines = 20 } = {}) {
  const ids: string[] = []
  await page.route(/\/rest\/getLyricsBySongId(\?|$)/, (route) => {
    const id = new URL(route.request().url()).searchParams.get('id')!
    ids.push(id)
    const structuredLyrics = synced.includes(id)
      ? [{ lang: 'jpn', synced: true, line: Array.from({ length: lines }, (_, i) => ({ start: i * 3000, value: i === 0 ? '' : `時刻付きの歌詞 ${i} 行目` })) }]
      : plain.includes(id)
        ? [{ lang: 'jpn', synced: false, line: Array.from({ length: lines }, (_, i) => ({ value: `時刻なしの歌詞 ${i + 1} 行目` })) }]
        : []
    return route.fulfill({ json: { 'subsonic-response': { status: 'ok', version: '1.16.1', lyricsList: { structuredLyrics } } } })
  })
  return { ids }
}
