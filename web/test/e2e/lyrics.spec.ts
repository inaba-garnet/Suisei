import { expect, test } from '@playwright/test'
import { mockApi, mockLibrary, mockLyrics, mockStream } from './api'

test('右の欄では、歌詞のある曲でだけ歌詞を開け、ジャケットの場所に入れ替えて出す。キューとは同時に開かない', async ({ page, isMobile }) => {
  test.skip(isMobile, 'PC の画面だけ')
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 4 })
  await mockStream(page, { seconds: 120 })
  const { ids } = await mockLyrics(page, { synced: ['al-1-2'] })
  await page.setViewportSize({ width: 1500, height: 900 })
  await page.goto('/library/albums/al-1')

  const panel = page.getByTestId('player-panel')
  const button = panel.getByRole('button', { name: '歌詞', exact: true })
  await expect(button).toBeDisabled()

  // 歌詞のない曲では押せない
  await page.getByTestId('songs').getByText('一曲目').click()
  await expect.poll(() => ids).toEqual(['al-1-1'])
  await expect(button).toBeDisabled()

  await panel.getByRole('button', { name: '次の曲' }).click()
  await expect(button).toBeEnabled()
  await button.click()
  await expect(button).toHaveAttribute('aria-pressed', 'true')
  await expect(panel.getByRole('heading', { name: '歌詞' })).toBeVisible()
  await expect(panel.getByTestId('lyrics')).toContainText('時刻付きの歌詞 1 行目')

  // キューを開くと歌詞を閉じ、歌詞を開くとキューを閉じる
  const queue = panel.getByRole('button', { name: 'キュー', exact: true })
  await queue.click()
  await expect(panel.getByTestId('player-queue')).toBeVisible()
  await expect(panel.getByTestId('lyrics')).toHaveCount(0)
  await expect(button).toHaveAttribute('aria-pressed', 'false')
  await button.click()
  await expect(panel.getByTestId('lyrics')).toBeVisible()
  await expect(queue).toHaveAttribute('aria-pressed', 'false')

  // 歌詞のない曲に移ると閉じてジャケットに戻す。戻った曲の歌詞は取り直さない
  await panel.getByRole('button', { name: '次の曲' }).click()
  await expect(panel.getByRole('heading', { name: '再生中' })).toBeVisible()
  await expect(button).toBeDisabled()
  await panel.getByRole('button', { name: '前の曲' }).click()
  await expect(panel).toContainText('二曲目')
  await expect(button).toBeEnabled()
  expect(ids).toEqual(['al-1-1', 'al-1-2', 'al-1-3'])
})

test('時刻付きの歌詞は鳴っている行を示し、行を押すとその位置へ移る', async ({ page, isMobile }) => {
  test.skip(isMobile, 'PC の画面だけ')
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 2 })
  await mockStream(page, { seconds: 120 })
  await mockLyrics(page, { synced: ['al-1-1'] })
  await page.setViewportSize({ width: 1500, height: 900 })
  await page.goto('/library/albums/al-1')

  await page.getByTestId('songs').getByText('一曲目').click()
  const panel = page.getByTestId('player-panel')
  await panel.getByRole('button', { name: '歌詞', exact: true }).click()
  const lyrics = panel.getByTestId('lyrics')

  // 本文が空の行（前奏）は「♪」と出す
  await expect(lyrics.getByRole('button').first()).toHaveText('♪')

  // 8 行目（24 秒）を押すと、その位置へ移ってその行を鳴っている行にし、上から 3 分の 1 あたりへ送る
  const line = lyrics.getByRole('button', { name: '時刻付きの歌詞 8 行目' })
  await line.click()
  await expect(line).toHaveAttribute('aria-current', 'true')
  const seconds = Number(await panel.getByRole('slider', { name: '再生位置' }).getAttribute('aria-valuenow'))
  expect(seconds).toBeGreaterThanOrEqual(24)
  expect(seconds).toBeLessThan(27)
  await expect.poll(async () => {
    const box = (await lyrics.boundingBox())!
    const at = (await line.boundingBox())!
    return (at.y - box.y) / box.height
  }).toBeLessThan(0.45)
})

test('時刻なしの歌詞は、押せない行を並べるだけにする', async ({ page, isMobile }) => {
  test.skip(isMobile, 'PC の画面だけ')
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 2 })
  await mockStream(page)
  await mockLyrics(page, { plain: ['al-1-1'], lines: 5 })
  await page.setViewportSize({ width: 1500, height: 900 })
  await page.goto('/library/albums/al-1')

  await page.getByTestId('songs').getByText('一曲目').click()
  const panel = page.getByTestId('player-panel')
  await panel.getByRole('button', { name: '歌詞', exact: true }).click()
  const lyrics = panel.getByTestId('lyrics')
  await expect(lyrics.getByRole('listitem')).toHaveCount(5)
  await expect(lyrics.getByRole('button')).toHaveCount(0)
  await expect(lyrics).toContainText('時刻なしの歌詞 5 行目')
})

test('スマホの再生画面でも歌詞を開け、閉じて開き直すとジャケットの表示から始める', async ({ page, isMobile }) => {
  test.skip(!isMobile, 'スマホの画面だけ')
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 2 })
  await mockStream(page)
  await mockLyrics(page, { synced: ['al-1-1'] })
  await page.goto('/library/albums/al-1')

  await page.getByTestId('songs').getByText('一曲目').click()
  const open = page.getByTestId('player-bar').getByRole('button', { name: '再生画面を開く' })
  await open.click()
  const sheet = page.getByRole('dialog', { name: '再生画面' })
  await expect.poll(async () => (await sheet.boundingBox())?.y).toBe(16)
  await sheet.getByRole('button', { name: '歌詞', exact: true }).click()
  await expect(sheet.getByRole('heading', { name: '歌詞' })).toBeVisible()
  await expect(sheet.getByTestId('lyrics')).toContainText('時刻付きの歌詞 1 行目')

  await sheet.getByRole('button', { name: '再生画面を閉じる' }).click()
  await open.click()
  await expect(sheet.getByRole('heading', { name: '再生中' })).toBeVisible()
  await expect(sheet.getByTestId('lyrics')).toHaveCount(0)
})
