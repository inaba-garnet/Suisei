import { expect, test } from '@playwright/test'
import { mockApi, mockGenres } from './api'

test('ジャンルを名前順に並べ、曲の多い順に切り替えると選んだ順を残す', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockGenres(page)
  await page.goto('/library/genres')

  const rows = page.getByTestId('genres').getByRole('link')
  await expect(rows).toHaveCount(3)
  await expect(rows.first()).toContainText('J-Pop')
  await expect(rows.nth(1)).toContainText('アニソン12 曲 · 2 枚')

  const sort = page.getByRole('combobox', { name: '並び順' })
  await sort.selectOption('songs')
  await expect(rows.first()).toContainText('アニソン')

  await page.reload()
  await expect(sort).toHaveValue('songs')
  await expect(rows.first()).toContainText('アニソン')
})

test('ジャンルを押すと、そのジャンルの曲を並び順を選んで並べ、「戻る」で一覧に戻る', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  const { requests } = await mockGenres(page)
  await page.goto('/library/genres')
  await page.getByTestId('genres').getByRole('link', { name: /アニソン/ }).click()

  await expect(page).toHaveURL(`/library/genres/${encodeURIComponent('アニソン')}`)
  await expect(page.getByRole('heading', { name: 'アニソン' })).toBeVisible()
  const songs = page.getByTestId('songs')
  await expect(songs.getByText('title:アニソン の曲 1')).toBeVisible()
  await expect(songs.getByRole('listitem')).toHaveCount(3)
  expect(requests[0]?.get('genre')).toBe('アニソン')
  expect(requests[0]?.get('songSort')).toBe('title')
  expect(requests[0]?.get('count')).toBe('100')

  const sort = page.getByRole('combobox', { name: '並び順' })
  await sort.selectOption('artist')
  await expect(songs.getByText('artist:アニソン の曲 1')).toBeVisible()
  expect(requests.at(-1)?.get('songSort')).toBe('artist')

  await page.getByRole('button', { name: '戻る', exact: true }).click()
  await expect(page).toHaveURL('/library/genres')
})
