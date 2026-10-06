import { expect, test } from '@playwright/test'
import { mockApi, mockSearch, mockSettings, mockSpotify, SPOTIFY_REDIRECT_URI } from './api'

test.beforeEach(async ({ page }) => {
  await mockApi(page, { loggedIn: true })
})

test('Client ID がなければ、欄と登録の手順だけを出し、入れると接続できる', async ({ page }) => {
  const { saved } = await mockSettings(page)
  await page.goto('/settings')
  await expect(page.getByTestId('spotify-redirect-uri')).toHaveText(SPOTIFY_REDIRECT_URI)
  await expect(page.getByRole('button', { name: 'Spotify に接続' })).toHaveCount(0)

  const input = page.getByLabel('Client ID')
  await input.fill('https://developer.spotify.com/')
  await page.getByRole('button', { name: '保存', exact: true }).click()
  await expect(page.getByRole('alert')).toContainText('Client ID の形が違います')

  // 保存すると、接続の操作を出す
  await mockSpotify(page)
  await input.fill('0123456789abcdef')
  await page.getByRole('button', { name: '保存', exact: true }).click()
  await expect(page.getByRole('button', { name: 'Spotify に接続' })).toBeVisible()
  expect(saved.at(-1)?.spotifyClientId).toBe('https://developer.spotify.com/')
})

test('接続中に Client ID を変えるときは確かめ、変えたら接続が切れる', async ({ page }) => {
  const spotify = await mockSpotify(page, { connected: true })
  await page.goto('/settings')
  await expect(page.getByTestId('spotify-counts')).toBeVisible()

  await page.getByLabel('Client ID').fill('other')
  page.once('dialog', dialog => dialog.dismiss())
  await page.getByRole('button', { name: '保存', exact: true }).click()
  expect(spotify.state.connected).toBe(true)

  page.once('dialog', dialog => dialog.accept())
  await page.getByRole('button', { name: '保存', exact: true }).click()
  await expect(page.getByRole('button', { name: 'Spotify に接続' })).toBeVisible()
  expect(spotify.state.connected).toBe(false)
})

test('認可の後の URL を貼り付けて接続し、すぐに取り込む', async ({ page }) => {
  const spotify = await mockSpotify(page)
  await page.goto('/settings')
  await expect(page.getByTestId('spotify-redirect-uri')).toHaveText(SPOTIFY_REDIRECT_URI)

  // 認可の画面は新しいタブで開く
  const popup = page.waitForEvent('popup')
  await page.getByRole('button', { name: 'Spotify に接続' }).click()
  await expect((await popup)).toHaveURL(/accounts\.spotify\.com\/authorize/)

  const input = page.getByLabel('許可した後に開けなかったページの URL を貼り付けてください')
  await input.fill(`${SPOTIFY_REDIRECT_URI}?code=c&state=old`)
  await page.getByRole('button', { name: '接続', exact: true }).click()
  await expect(page.getByRole('alert')).toContainText('古い URL です')
  expect(spotify.state.connected).toBe(false)

  await input.fill(`${SPOTIFY_REDIRECT_URI}?code=c&state=st`)
  await page.getByRole('button', { name: '接続', exact: true }).click()
  await expect(page.getByTestId('spotify-counts')).toContainText('3 曲のうち、1 曲')
  await expect(page.getByTestId('spotify-last-sync')).toContainText('1 曲をお気に入りにしました')
  expect(spotify.state.syncs).toBe(1)
  await expect(page.getByTestId('spotify-unmatched')).toHaveText('2 曲')
})

test('接続を解除する', async ({ page }) => {
  const spotify = await mockSpotify(page, { connected: true })
  await page.goto('/settings')
  await page.getByRole('button', { name: '接続を解除' }).click()
  await expect(page.getByRole('button', { name: 'Spotify に接続' })).toBeVisible()
  expect(spotify.state.connected).toBe(false)
  // 対応表は残るので、未対応の曲にはまだ移れる
  await expect(page.getByTestId('spotify-unmatched')).toHaveText('2 曲')
})

test('未対応の曲にライブラリの曲を選んで対応を付ける', async ({ page }) => {
  const spotify = await mockSpotify(page, { connected: true })
  const search = await mockSearch(page)
  await page.goto('/settings')
  await page.getByRole('link', { name: /未対応の曲/ }).click()
  await expect(page).toHaveURL(/\/settings\/spotify$/)

  const row = page.locator('[data-spotify-id="sp-2"]')
  await expect(row).toContainText('歌手B / 歌手C')
  await row.getByRole('button', { name: '選ぶ' }).click()
  // Spotify の曲名で探し始める
  await expect(row.getByRole('searchbox', { name: 'ライブラリの曲を検索' })).toHaveValue('ひかりの曲')
  await expect.poll(() => search.requests.at(-1)?.get('query')).toBe('ひかりの曲')
  await row.getByRole('button', { name: /ひかりの曲 001/ }).click()

  await expect(page.getByRole('status')).toHaveText('「ひかりの曲 001」をお気に入りにしました')
  await expect(row).toHaveCount(0)
  expect(spotify.links).toEqual([{ id: 'sp-2', trackId: 'tr-1' }])
})

test('ライブラリにない曲を対応させない', async ({ page }) => {
  const spotify = await mockSpotify(page, { connected: true })
  await page.goto('/settings/spotify')
  const row = page.locator('[data-spotify-id="sp-3"]')
  await row.getByRole('button', { name: '対応させない' }).click()
  await expect(row.getByText('対応させない', { exact: true })).toBeVisible()
  await expect(row.getByRole('button', { name: '対応させない' })).toHaveCount(0)
  expect(spotify.links).toEqual([{ id: 'sp-3', trackId: null }])
})
