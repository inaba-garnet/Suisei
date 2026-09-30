import type { Page } from '@playwright/test'
import { expect, test } from '@playwright/test'
import { mockApi, mockArtists } from './api'

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
  const { calls } = await mockArtists(page)
  await page.goto('/library/artists')
  await expect(page.getByRole('link', { name: /アのアーティスト 0/ })).toBeVisible()

  await scrollTo(page, 600)
  const before = await scrollTop(page)
  const link = page.getByTestId('artist-list').getByRole('link').nth(3)
  const name = (await link.locator('span span').first().textContent())!
  await link.dispatchEvent('click')

  await expect(page.getByRole('heading', { name })).toBeVisible()
  await expect(page.getByTestId('album-grid').getByRole('link')).toHaveCount(2)
  await expect(page.getByTestId('album-grid')).toContainText(`${name} の一枚目`)

  await page.getByRole('button', { name: 'アーティストに戻る' }).click()
  await expect(page).toHaveURL('/library/artists')
  await expect.poll(() => scrollTop(page)).toBe(before)
  expect(calls.artists).toBe(1)
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
  await expect(page.getByTestId('album-grid')).toBeVisible()
  await page.getByRole('button', { name: 'アーティストに戻る' }).click()
  await expect(page.getByRole('radio', { name: 'アーティスト', exact: true })).toHaveAttribute('aria-checked', 'true')
  await expect(page.getByRole('link', { name: /^artist:アのアーティスト 0/ })).toBeVisible()
  expect(roles).toEqual(['albumartist', 'artist'])

  // ライブラリから開き直すと、アルバムアーティストに戻る
  await page.goto('/library')
  await page.getByRole('main').getByRole('link', { name: 'アーティスト' }).click()
  await expect(page.getByRole('radio', { name: 'アルバムアーティスト' })).toHaveAttribute('aria-checked', 'true')
})

test('作曲家の一覧は作曲、作詞、編曲をまとめて求め、詳細から作曲家の一覧に戻る', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  const { roles } = await mockArtists(page)
  await page.goto('/library/composers')
  await expect(page.getByRole('heading', { name: '作曲家' })).toBeVisible()
  const link = page.getByRole('link', { name: /^composer,lyricist,arranger:アのアーティスト 0/ })
  await expect(link).toBeVisible()
  expect(roles).toEqual(['composer,lyricist,arranger'])

  await link.click()
  await expect(page.getByTestId('album-grid')).toBeVisible()
  await page.getByRole('button', { name: '作曲家に戻る' }).click()
  await expect(page).toHaveURL('/library/composers')
})
