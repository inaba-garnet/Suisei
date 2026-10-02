import { expect, test } from '@playwright/test'
import { mockApi, mockLibrary, mockStream, mockTracks } from './api'

test('アルバムの「すべて再生」で一曲目から鳴らし、終われば次の曲に進み、最後の曲で止まる', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3 })
  const { ids } = await mockStream(page, { seconds: 0.5 })
  await page.goto('/library/albums/al-1')

  const bar = page.getByTestId('player-bar')
  await expect(bar).toContainText('再生していません')
  await expect(bar.getByRole('button', { name: '再生' })).toBeDisabled()

  await page.getByRole('button', { name: 'すべて再生' }).click()
  await expect(bar).toContainText('一曲目')
  await expect.poll(() => ids).toEqual(['al-1-1', 'al-1-2'])
  await expect(bar).toContainText('二曲目')
  // 最後の曲が終わったら、その曲を出したまま止める
  await expect(bar.getByRole('button', { name: '再生' })).toBeEnabled()
  await expect(page.getByTestId('songs').getByText('二曲目')).toHaveClass(/text-accent-base/)
})

test('曲の行を押すとその曲から鳴らし、一時停止と再開ができ、画面を移っても途切れない', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockTracks(page)
  const { ids } = await mockStream(page)
  await page.goto('/library/tracks')

  await page.getByTestId('songs').getByText('title:曲 002').click()
  const bar = page.getByTestId('player-bar')
  await expect(bar).toContainText('title:曲 002')
  const pause = bar.getByRole('button', { name: '一時停止' })
  await expect(pause).toBeVisible()
  expect(ids).toEqual(['tr-002'])

  await pause.click()
  await expect(bar.getByRole('button', { name: '再生' })).toBeVisible()
  await bar.getByRole('button', { name: '再生' }).click()
  await expect(pause).toBeVisible()

  // 画面を移っても同じ曲を鳴らし続け、読み直さない
  await page.getByRole('link', { name: '設定' }).click()
  await expect(page).toHaveURL('/settings')
  await expect(bar).toContainText('title:曲 002')
  await expect(pause).toBeVisible()
  expect(ids).toEqual(['tr-002'])
})

test('前後の曲に移り、前の曲は先まで進んでいれば曲の頭に戻す', async ({ page, isMobile }) => {
  test.skip(isMobile, '前後の曲のボタンは PC だけ')
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 3 })
  const { ids } = await mockStream(page)
  await page.goto('/library/albums/al-1')

  const bar = page.getByTestId('player-bar')
  await page.getByTestId('songs').getByText('二曲目').click()
  await expect(bar).toContainText('二曲目')
  await bar.getByRole('button', { name: '次の曲' }).click()
  await expect(bar).toContainText('曲 3')
  // キューの最後なので次の曲はない
  await expect(bar.getByRole('button', { name: '次の曲' })).toBeDisabled()

  await bar.getByRole('button', { name: '前の曲' }).click()
  await expect(bar).toContainText('二曲目')
  expect(ids).toEqual(['al-1-2', 'al-1-3', 'al-1-2'])

  // 3 秒より先まで進んでいれば、前の曲には移らず頭に戻す
  await expect(bar).toContainText('0:04 /', { timeout: 10_000 })
  await bar.getByRole('button', { name: '前の曲' }).click()
  await expect(bar).toContainText('0:00 /')
  await expect(bar).toContainText('二曲目')
  expect(ids).toHaveLength(3)
})

test('鳴らせない曲は再生バーで知らせる', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3 })
  await mockStream(page, { fail: true })
  await page.goto('/library/albums/al-1')

  await page.getByRole('button', { name: 'すべて再生' }).click()
  const bar = page.getByTestId('player-bar')
  await expect(bar).toContainText('再生できませんでした')
  await expect(bar.getByRole('button', { name: '再生' })).toBeVisible()
})

test('広い画面では右の欄に再生中の曲を出して操作する', async ({ page, isMobile }) => {
  test.skip(isMobile, 'PC の画面だけ')
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3 })
  await mockStream(page)
  await page.setViewportSize({ width: 1500, height: 900 })
  await page.goto('/library/albums/al-1')

  const panel = page.getByTestId('player-panel')
  await expect(panel).toContainText('再生していません')
  // シャッフルとリピートは開発モードのときだけ
  await expect(panel.getByRole('button', { name: 'シャッフル' })).toHaveCount(0)

  await page.getByRole('button', { name: 'すべて再生' }).click()
  await expect(panel).toContainText('一曲目')
  await expect(panel).toContainText('アーティスト 1')
  await panel.getByRole('button', { name: '一時停止' }).click()
  await expect(panel.getByRole('button', { name: '再生' })).toBeVisible()
  await panel.getByRole('button', { name: '次の曲' }).click()
  await expect(panel).toContainText('二曲目')
})
