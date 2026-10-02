import type { Page } from '@playwright/test'
import { expect, test } from '@playwright/test'
import { mockApi, mockStar, mockTracks } from './api'

const scrollTop = (page: Page) => page.getByTestId('scroller').evaluate(el => el.scrollTop)
const scrollTo = (page: Page, top: number) => page.getByTestId('scroller').evaluate((el, top) => {
  el.scrollTop = top
}, top)

test('全曲を曲名順に 100 曲ずつ読み足し、見える分だけ描く', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  const { requests } = await mockTracks(page)
  await page.goto('/library/tracks')

  const songs = page.getByTestId('songs')
  await expect(songs.getByText('title:曲 000')).toBeVisible()
  await expect(songs).toContainText('アーティスト 000 · アルバム 000')
  expect(requests[0]?.get('query')).toBe('')
  expect(requests[0]?.get('songSort')).toBe('title')
  expect(requests[0]?.get('songCount')).toBe('100')
  expect(requests[0]?.get('artistCount')).toBe('0')
  expect(await songs.getByRole('listitem').count()).toBeLessThan(100)

  // 末尾まで送ると続きを読み足す
  await scrollTo(page, 1e6)
  await expect.poll(() => requests.map(r => r.get('songOffset'))).toContain('100')
  await scrollTo(page, 1e6)
  await expect.poll(() => requests.map(r => r.get('songOffset'))).toContain('200')
  await scrollTo(page, 1e6)
  await expect(songs.getByText('title:曲 249')).toBeVisible()
})

test('並び順を切り替えるとサーバーに順を頼んで先頭から読み直し、選んだ順を残す', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  const { requests } = await mockTracks(page)
  await page.goto('/library/tracks')
  const sort = page.getByRole('combobox', { name: '並び順' })
  await expect(sort).toHaveValue('title')
  await expect(page.getByText('title:曲 000')).toBeVisible()

  await scrollTo(page, 1500)
  await sort.selectOption('album')
  await expect(page.getByText('album:曲 000')).toBeVisible()
  expect(requests.at(-1)?.get('songSort')).toBe('album')
  expect(requests.at(-1)?.get('songOffset')).toBe('0')
  expect(await scrollTop(page)).toBe(0)

  await page.reload()
  await expect(sort).toHaveValue('album')
  await expect(page.getByText('album:曲 000')).toBeVisible()
  await expect(sort.getByRole('option')).toHaveText(['曲名順', 'アルバム順', 'アーティスト順'])
})

test('曲のハートで付け外しし、行が描き直されても付け外しを保つ', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockTracks(page)
  const { calls } = await mockStar(page)
  await page.goto('/library/tracks')

  const heart = page.getByRole('button', { name: 'title:曲 000をお気に入りにする' })
  await expect(heart).toHaveAttribute('aria-pressed', 'false')
  await heart.click()
  await expect(heart).toHaveAttribute('aria-pressed', 'true')
  await expect.poll(() => calls.at(-1)?.endpoint).toBe('star')
  await expect.poll(() => calls.at(-1)?.params.get('id')).toBe('tr-000')

  // 見えない所まで送って行を消し、戻って描き直す
  await scrollTo(page, 1e6)
  await expect(heart).toHaveCount(0)
  await scrollTo(page, 0)
  await expect(heart).toHaveAttribute('aria-pressed', 'true')
})
