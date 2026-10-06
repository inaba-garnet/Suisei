import type { Page } from '@playwright/test'
import { expect, test } from '@playwright/test'
import { mockApi, mockArtists, mockStar, mockStream } from './api'

const scrollTop = (page: Page) => page.getByTestId('scroller').evaluate(el => el.scrollTop)
const scrollTo = (page: Page, top: number) => page.getByTestId('scroller').evaluate((el, top) => {
  el.scrollTop = top
}, top)

test('アーティストを読みの行の見出しで区切って並べ、今いる行の見出しを帯の下に残す', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockArtists(page)
  await page.goto('/library/artists')

  const list = page.getByTestId('artist-list')
  await expect(list.getByRole('heading', { name: 'ア', exact: true })).toBeVisible()
  await expect(list.getByRole('link', { name: /アのアーティスト 0/ })).toBeVisible()
  await expect(list.getByRole('link', { name: /アのアーティスト 0/ })).toContainText('アルバム 2 枚')

  // カの行の途中までスクロールすると、カの見出しが帯のすぐ下に残る
  await scrollTo(page, 32 + 56 * 12 + 32 + 56 * 5)
  const heading = list.getByRole('heading', { name: 'カ', exact: true })
  await expect(heading).toBeVisible()
  const header = (await page.getByTestId('page-header').boundingBox())!
  await expect.poll(async () => Math.round((await heading.boundingBox())!.y)).toBe(Math.round(header.y + header.height))
})

test('アーティストの詳細でアルバムを並べ、「戻る」と読み直さずに元の位置に戻る', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  const { calls, songs } = await mockArtists(page)
  await page.goto('/library/artists')
  await expect(page.getByRole('link', { name: /アのアーティスト 0/ })).toBeVisible()

  await scrollTo(page, 600)
  const before = await scrollTop(page)
  const link = page.getByTestId('artist-list').getByRole('link').nth(3)
  const name = (await link.locator('span span').first().textContent())!
  await link.dispatchEvent('click')

  await expect(page.getByRole('heading', { name })).toBeVisible()
  await expect(page).toHaveURL(/\?view=albums$/)
  // アルバムアーティストになっているアルバムだけを並べる
  await expect(page.getByTestId('album-grid').getByRole('link')).toHaveCount(2)
  await expect(page.getByTestId('album-grid')).toContainText(`${name} の一枚目`)
  await expect(page.getByTestId('album-grid')).not.toContainText('客演のアルバム')
  await expect(page.getByTestId('songs')).toHaveCount(0)

  await page.getByRole('button', { name: '戻る', exact: true }).click()
  await expect(page).toHaveURL('/library/artists')
  await expect.poll(() => scrollTop(page)).toBe(before)
  expect(calls.artists).toBe(1)
  expect(songs).toEqual([''])
})

test('アーティストの一覧でアルバムアーティストと曲のアーティストを切り替え、戻ると選んでいた方に戻る', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  const { roles } = await mockArtists(page)
  await page.goto('/library/artists')
  await expect(page.getByRole('radio', { name: 'アルバムアーティスト' })).toHaveAttribute('aria-checked', 'true')
  await expect(page.getByRole('link', { name: /^アのアーティスト 0/ })).toBeVisible()
  expect(roles).toEqual(['albumartist'])

  await page.getByRole('radio', { name: 'アーティスト', exact: true }).click()
  const link = page.getByRole('link', { name: /^artist:アのアーティスト 0/ })
  await expect(link).toBeVisible()
  expect(roles).toEqual(['albumartist', 'artist'])

  await link.click()
  await expect(page.getByTestId('songs')).toBeVisible()
  await page.getByRole('button', { name: '戻る', exact: true }).click()
  await expect(page.getByRole('radio', { name: 'アーティスト', exact: true })).toHaveAttribute('aria-checked', 'true')
  await expect(page.getByRole('link', { name: /^artist:アのアーティスト 0/ })).toBeVisible()
  expect(roles).toEqual(['albumartist', 'artist'])

  // ライブラリから開き直すと、アルバムアーティストに戻る
  await page.goto('/library')
  await page.getByRole('main').getByRole('link', { name: 'アーティスト' }).click()
  await expect(page.getByRole('radio', { name: 'アルバムアーティスト' })).toHaveAttribute('aria-checked', 'true')
})

test('作曲家の一覧は作曲、作詞、編曲をまとめて求め、詳細は曲をアルバムで区切らずに並べ、作曲家の一覧に戻る', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  const { roles, songs } = await mockArtists(page)
  await page.goto('/library/composers')
  await expect(page.getByRole('heading', { name: '作曲家' })).toBeVisible()
  const link = page.getByRole('link', { name: /^composer,lyricist,arranger:アのアーティスト 0/ })
  await expect(link).toBeVisible()
  expect(roles).toEqual(['composer,lyricist,arranger'])

  await link.click()
  await expect(page).toHaveURL(/\?view=composer$/)
  await expect(page.getByTestId('songs').getByRole('listitem')).toHaveCount(3)
  await expect(page.getByTestId('songs')).toContainText('客演のアルバム')
  await expect(page.getByTestId('song-group')).toHaveCount(0)
  await expect(page.getByTestId('album-grid')).toHaveCount(0)
  expect(songs).toEqual(['composer,lyricist,arranger'])

  await page.getByRole('button', { name: '戻る', exact: true }).click()
  await expect(page).toHaveURL('/library/composers')
})

test('アーティストの詳細から開いたアルバムの「戻る」は、アーティストの詳細に戻る', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockArtists(page)
  await page.route(/\/rest\/getAlbum(\?|$)/, (route) => {
    const id = new URL(route.request().url()).searchParams.get('id')!
    return route.fulfill({ json: { 'subsonic-response': { status: 'ok', version: '1.16.1', album: { id, name: 'そのアルバム', songCount: 0, duration: 0, song: [] } } } })
  })
  await page.goto('/library/artists/ar-ア-0')
  await page.getByTestId('album-grid').getByRole('link').first().click()
  await expect(page.getByRole('heading', { name: 'そのアルバム' })).toBeVisible()

  await page.getByRole('button', { name: '戻る', exact: true }).click()
  await expect(page).toHaveURL(/\/library\/artists\/ar-/)
  await expect(page.getByRole('heading', { name: 'アのアーティスト 0' })).toBeVisible()
})

test('アーティストの一覧から開いた詳細は、アルバムを横に並べ、曲をアルバムごとの見出しで区切る', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  const { songs } = await mockArtists(page)
  await page.goto('/library/artists')
  await page.getByRole('radio', { name: 'アーティスト', exact: true }).click()
  await page.getByRole('link', { name: /^artist:アのアーティスト 0/ }).click()
  await expect(page).toHaveURL(/\?view=tracks$/)

  // アルバムアーティストになっているアルバムだけを横に並べる
  const shelf = page.getByTestId('album-shelf')
  await expect(shelf.getByRole('link')).toHaveCount(2)
  await expect(shelf).not.toContainText('客演のアルバム')

  // 曲はアルバムごとの見出しで区切る
  const groups = page.getByTestId('song-group')
  await expect(groups).toHaveCount(2)
  await expect(groups.nth(0)).toContainText('客演のアルバム')
  await expect(page.getByTestId('songs')).toContainText('ほかの人 feat.')
  expect(songs).toEqual(['artist'])

  // 再読み込みしても同じ表示にする
  await page.reload()
  await expect(page.getByTestId('song-group')).toHaveCount(2)
})

test('アーティストの曲の行を押すと、見出しごとに並んだ順で押した曲から鳴らし、ハートで付け外しする', async ({ page, isMobile }) => {
  test.skip(isMobile, '次の曲のボタンは PC だけ')
  await mockApi(page, { loggedIn: true })
  await mockArtists(page)
  const { ids } = await mockStream(page)
  const { calls } = await mockStar(page)
  await page.goto('/library/artists/ar-ア-0?view=tracks')

  // 新しい順なので、客演のアルバム（2015）の曲の次に一枚目（2010）の曲が並ぶ
  const songs = page.getByTestId('songs')
  await songs.getByText('客演の曲', { exact: true }).click()
  const bar = page.getByTestId('player-bar')
  await expect(bar).toContainText('客演の曲')
  await bar.getByRole('button', { name: '次の曲' }).click()
  await expect(bar).toContainText('一曲目')
  await expect.poll(() => ids).toEqual(['s-3', 's-1'])

  const heart = songs.getByRole('button', { name: '二曲目をお気に入りにする' })
  await heart.click()
  await expect(heart).toHaveAttribute('aria-pressed', 'true')
  await expect.poll(() => calls.at(-1)?.params.get('id')).toBe('s-2')
})

test('アーティストの詳細の並び順は既定で新しい順にし、選んだ順を表示ごとに残す', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  const { index } = await mockArtists(page)
  const name = index[0]!.artist[0]!.name
  const shelf = page.getByTestId('album-shelf').getByRole('link')
  const groups = page.getByTestId('song-group')
  const sort = page.getByRole('combobox', { name: '並び順' })

  await page.goto('/library/artists/ar-ア-0?view=tracks')
  await expect(sort).toHaveValue('newest')
  await expect(shelf.first()).toContainText(`${name} の二枚目`)
  await expect(groups.first()).toContainText('客演のアルバム')

  // 古い順にすると、横に並べたアルバムと曲の見出しを同じ順で入れ替える
  await sort.selectOption('oldest')
  await expect(shelf.first()).toContainText(`${name} の一枚目`)
  await expect(groups.first()).toContainText(`${name} の一枚目`)

  // 開き直しても選んだ順を保ち、ほかの表示には持ち込まない
  await page.reload()
  await expect(sort).toHaveValue('oldest')
  await page.goto('/library/artists/ar-ア-0?view=composer')
  await expect(sort).toHaveValue('newest')
  const titles = page.getByTestId('songs').getByRole('listitem')
  await expect(titles.first()).toContainText('客演の曲')
  await sort.selectOption('name')
  await expect(titles.first()).toContainText('一曲目')
  await expect(sort.getByRole('option', { name: '曲名順' })).toHaveCount(1)
})

test('お気に入りに絞り込むと、お気に入りのアーティストだけを読みの行の見出しごと残す', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  const { calls } = await mockArtists(page)
  await page.goto('/library/artists')
  const list = page.getByTestId('artist-list')
  await expect(list.getByRole('link', { name: /アのアーティスト 0/ })).toBeVisible()

  await page.getByRole('button', { name: 'お気に入り', exact: true }).click()
  await expect(list.getByRole('link')).toHaveCount(2)
  await expect(list.getByRole('heading')).toHaveText(['カ'])
  // 読み直さずに絞り込む
  expect(calls.artists).toBe(1)

  // 作曲家の一覧にも同じ切り替えがある
  await page.goto('/library/composers')
  await page.getByRole('button', { name: 'お気に入り', exact: true }).click()
  await expect(list.getByRole('link')).toHaveCount(2)
})

test('曲と作曲の両方で関わった人は詳細で表示を切り替え、「戻る」で開く前の画面に戻る', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  const { songs } = await mockArtists(page)
  await page.goto('/library/composers')
  await page.getByRole('link', { name: /^composer,lyricist,arranger:アのアーティスト 0/ }).click()
  await expect(page).toHaveURL(/\?view=composer$/)
  const views = page.getByRole('radiogroup', { name: '表示' })
  await expect(views.getByRole('radio')).toHaveText(['曲', '作曲'])
  await expect(views.getByRole('radio', { name: '作曲' })).toHaveAttribute('aria-checked', 'true')

  await views.getByRole('radio', { name: '曲', exact: true }).click()
  await expect(page).toHaveURL(/\?view=tracks$/)
  await expect(page.getByTestId('song-group')).toHaveCount(2)
  expect(songs).toEqual(['composer,lyricist,arranger', 'artist'])

  // 切り替えは履歴に積まない
  await page.getByRole('button', { name: '戻る', exact: true }).click()
  await expect(page).toHaveURL('/library/composers')
})

test('片方の役割だけの人には表示の切り替えを出さない', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockArtists(page)
  await page.goto('/library/artists/ar-ア-1?view=tracks')
  await expect(page.getByTestId('song-group')).toHaveCount(2)
  await expect(page.getByRole('radiogroup', { name: '表示' })).toHaveCount(0)
})
