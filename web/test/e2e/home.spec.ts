import type { Page } from '@playwright/test'
import { expect, test } from '@playwright/test'
import { mockApi, mockLibrary, mockPlaylists } from './api'

/** ホームの棚の偽物。`type` ごとに `counts` 枚のアルバムを返し、`fail` の種類は失敗させる。 */
async function mockHome(page: Page, { counts = { recent: 15, frequent: 15, starred: 15 } as Record<string, number>, fail = [] as string[] } = {}) {
  const requests: URLSearchParams[] = []
  await page.route(/\/rest\/getAlbumList2(\?|$)/, (route) => {
    const params = new URL(route.request().url()).searchParams
    requests.push(params)
    const type = params.get('type')!
    if (fail.includes(type)) {
      return route.fulfill({ status: 500 })
    }
    const size = Math.min(Number(params.get('size')), counts[type] ?? 0)
    const album = Array.from({ length: size }, (_, i) => ({ id: `al-${type}-${i}`, name: `${type} ${i}`, artist: 'ClariS', songCount: 2, duration: 400 }))
    return route.fulfill({ json: { 'subsonic-response': { status: 'ok', version: '1.16.1', albumList2: album.length ? { album } : {} } } })
  })
  await page.route(/\/rest\/getCoverArt(\?|$)/, route => route.fulfill({ status: 404 }))
  return { requests }
}

test('最近再生、よく聴く、お気に入りのアルバムとプレイリストを棚で 12 枚まで並べる', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockPlaylists(page, { count: 2 })
  const { requests } = await mockHome(page)
  await page.goto('/')

  const home = page.getByTestId('home')
  await expect(home.getByRole('heading', { level: 2 })).toHaveText(['最近再生したアルバム', 'よく聴くアルバム', 'お気に入りのアルバム', 'プレイリスト'])
  await expect(home.getByRole('region', { name: '最近再生したアルバム' }).getByRole('listitem')).toHaveCount(12)
  await expect(home.getByRole('region', { name: 'プレイリスト' }).getByRole('listitem')).toHaveCount(2)
  expect(requests.map(r => [r.get('type'), r.get('size')])).toEqual([['recent', '12'], ['frequent', '12'], ['starred', '12']])

  const more = home.getByRole('link', { name: 'すべて見る' })
  await expect(more).toHaveCount(4)
  await expect(more.nth(0)).toHaveAttribute('href', '/library/albums?sort=recent')
  await expect(more.nth(1)).toHaveAttribute('href', '/library/albums?sort=frequent')
  await expect(more.nth(2)).toHaveAttribute('href', '/library/albums?favorite=1')
  await expect(more.nth(3)).toHaveAttribute('href', '/playlists')
})

test('中身のない棚は見出しと案内を出し、「すべて見る」を出さない', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockPlaylists(page, { count: 0 })
  await mockHome(page, { counts: { recent: 0, frequent: 0, starred: 3 } })
  await page.goto('/')

  const recent = page.getByRole('region', { name: '最近再生したアルバム' })
  await expect(recent).toContainText('曲を再生すると、ここに並びます。')
  await expect(recent.getByRole('link', { name: 'すべて見る' })).toHaveCount(0)
  await expect(page.getByRole('region', { name: 'プレイリスト' })).toContainText('プレイリストを作ると、ここに並びます。')
  await expect(page.getByRole('region', { name: 'お気に入りのアルバム' }).getByRole('listitem')).toHaveCount(3)
})

test('一つの棚が失敗しても、ほかの棚は出す', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockPlaylists(page, { count: 1 })
  await mockHome(page, { fail: ['frequent'] })
  await page.goto('/')
  await expect(page.getByRole('region', { name: 'よく聴くアルバム' })).toContainText('読み込めませんでした')
  await expect(page.getByRole('region', { name: '最近再生したアルバム' }).getByRole('listitem')).toHaveCount(12)
  await expect(page.getByRole('button', { name: 'もう一度読み込む' })).toBeVisible()
})

test('「すべて見る」で開いた並び順は残さず、お気に入りは絞り込んだ状態で開く', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  const { requests } = await mockLibrary(page, { count: 9 })
  await page.goto('/library/albums?sort=recent')
  const sort = page.getByRole('combobox', { name: '並び順' })
  await expect(sort).toHaveValue('recent')
  await expect.poll(() => requests.at(-1)?.get('type')).toBe('recent')

  // ライブラリから開き直すと、残した並び順（既定の名前順）に戻る
  await page.goto('/library')
  await page.getByRole('main').getByRole('link', { name: 'アルバム' }).click()
  await expect(sort).toHaveValue('name')

  // 一覧で選び直した並び順は残す
  await page.goto('/library/albums?sort=frequent')
  await sort.selectOption('year')
  await page.goto('/library/albums')
  await expect(sort).toHaveValue('year')

  await page.goto('/library/albums?favorite=1')
  await expect(page.getByRole('button', { name: 'お気に入り', exact: true })).toHaveAttribute('aria-pressed', 'true')
  await expect.poll(() => requests.at(-1)?.get('starred')).toBe('true')
})

test('棚はスクロールバーを隠し、PC は見出しの矢印で送る。ほかのスクロールバーは細くする', async ({ page, isMobile }) => {
  await mockApi(page, { loggedIn: true })
  await mockPlaylists(page, { count: 1 })
  await mockHome(page)
  await page.goto('/')
  const shelf = page.getByRole('region', { name: '最近再生したアルバム' })
  const list = shelf.getByTestId('album-shelf')
  await expect(list).toHaveCSS('scrollbar-width', 'none')
  await expect(page.getByTestId('scroller')).toHaveCSS('scrollbar-width', 'thin')

  const previous = shelf.getByRole('button', { name: '最近再生したアルバムの前へ' })
  const next = shelf.getByRole('button', { name: '最近再生したアルバムの次へ' })
  if (isMobile) {
    // スマホは指で送るので矢印を出さない
    await expect(next).toBeHidden()
    return
  }
  await expect(previous).toBeDisabled()
  await next.click()
  await expect.poll(() => list.evaluate(el => el.scrollLeft)).toBeGreaterThan(0)
  await expect(previous).toBeEnabled()
  await previous.click()
  await expect.poll(() => list.evaluate(el => el.scrollLeft)).toBe(0)
  await expect(previous).toBeDisabled()

  // 棚に収まっているなら矢印を出さない
  await expect(page.getByRole('region', { name: 'プレイリスト' }).getByRole('button', { name: /の次へ$/ })).toHaveCount(0)
})
