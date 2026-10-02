import { expect, test } from '@playwright/test'
import { mockApi, mockFavorites, mockStar } from './api'

test('お気に入りの曲を新しい順に並べ、古い順は逆にし、それ以外はサーバーに順を頼む', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  const { requests } = await mockFavorites(page)
  await page.goto('/library/favorites')

  const songs = page.getByTestId('songs').getByRole('listitem')
  const sort = page.getByRole('combobox', { name: '並び順' })
  await expect(sort).toHaveValue('newest')
  await expect(songs).toHaveCount(3)
  await expect(songs.first()).toContainText('starred:お気に入り 1')
  expect(requests[0]?.has('songSort')).toBe(false)

  await sort.selectOption('oldest')
  await expect(songs.first()).toContainText('starred:お気に入り 3')

  await sort.selectOption('artist')
  await expect(songs.first()).toContainText('artist:お気に入り 1')
  expect(requests.at(-1)?.get('songSort')).toBe('artist')

  // 選んだ順は再読み込みの後も残す
  await page.reload()
  await expect(sort).toHaveValue('artist')
  await expect(sort.getByRole('option')).toHaveText(['新しい順', '古い順', 'タイトル順', 'アーティスト順', 'アルバム順'])
})

test('お気に入りの画面でハートを外しても行は残し、付け直せる', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockFavorites(page)
  const { calls } = await mockStar(page)
  await page.goto('/library/favorites')

  const songs = page.getByTestId('songs').getByRole('listitem')
  const heart = page.getByRole('button', { name: 'starred:お気に入り 1をお気に入りにする' })
  await expect(heart).toHaveAttribute('aria-pressed', 'true')
  await heart.click()
  await expect(heart).toHaveAttribute('aria-pressed', 'false')
  expect(calls.at(-1)?.endpoint).toBe('unstar')
  await expect(songs).toHaveCount(3)

  await heart.click()
  await expect(heart).toHaveAttribute('aria-pressed', 'true')
  expect(calls.at(-1)?.endpoint).toBe('star')
})
