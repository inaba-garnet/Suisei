import { expect, test } from '@playwright/test'
import { mockApi, mockLibrary, mockStream } from './api'

test('ブラウザが鳴らせない曲は変換して鳴らし、同じ形式の次の曲は最初から変換する', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 3 })
  const { requests } = await mockStream(page, { unplayable: true })
  await page.goto('/library/albums/al-1')

  await page.getByRole('button', { name: 'すべて再生' }).click()
  const bar = page.getByTestId('player-bar')
  await expect(bar.getByRole('button', { name: '一時停止' })).toBeVisible()
  await expect(bar).not.toContainText('再生できませんでした')
  await expect.poll(() => requests).toEqual([
    { id: 'al-1-1', format: null, timeOffset: null },
    { id: 'al-1-1', format: 'mp3', timeOffset: null },
  ])

  await page.getByTestId('songs').getByText('二曲目').click()
  await expect(bar).toContainText('二曲目')
  await expect(bar.getByRole('button', { name: '一時停止' })).toBeVisible()
  await expect.poll(() => requests.slice(2)).toEqual([
    { id: 'al-1-2', format: 'mp3', timeOffset: null },
  ])
})

test('変換して鳴らしている曲のシークは、その位置から変換し直させる', async ({ page, isMobile }) => {
  test.skip(isMobile, 'シークのバーをキーで動かすのは PC で確かめる')
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 3 })
  const { requests } = await mockStream(page, { unplayable: true })
  await page.goto('/library/albums/al-1')

  await page.getByRole('button', { name: 'すべて再生' }).click()
  const bar = page.getByTestId('player-bar')
  await expect(bar.getByRole('button', { name: '一時停止' })).toBeVisible()
  await expect.poll(() => requests.length).toBe(2)

  // 5 秒先に移ると、5 秒から頭出しした変換を頼み、進み具合は頭出しした位置から数える
  const slider = bar.getByRole('slider', { name: '再生位置' })
  await slider.focus()
  await page.keyboard.press('ArrowRight')
  await expect.poll(() => requests.at(-1)?.timeOffset).toMatch(/^5(\.\d+)?$/)
  await expect(bar).toContainText(/0:0[5-9] \//)
})
