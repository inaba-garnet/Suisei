import type { Locator, Page } from '@playwright/test'
import { expect, test } from '@playwright/test'
import { mockApi, mockLibrary } from './api'

const scrollTop = (page: Page) => page.getByTestId('scroller').evaluate(el => el.scrollTop)
const scrollTo = (page: Page, top: number) => page.getByTestId('scroller').evaluate((el, top) => {
  el.scrollTop = top
}, top)

test('アルバムを名前順に 100 件ずつ読み足し、見える分だけ描く', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  const { requests } = await mockLibrary(page)
  await page.goto('/library/albums')

  const grid = page.getByTestId('album-grid')
  await expect(grid.getByRole('link', { name: 'アルバム 000' })).toBeVisible()
  expect(requests[0]?.get('type')).toBe('alphabeticalByName')
  expect(requests[0]?.get('size')).toBe('100')
  expect(requests[0]?.get('offset')).toBe('0')
  // 仮想スクロールなので、読み込んだ 100 件より少ない数だけ描く
  expect(await grid.getByRole('link').count()).toBeLessThan(100)

  // 末尾まで送ると続きを読み足す
  await scrollTo(page, 1e6)
  await expect.poll(() => requests.map(r => r.get('offset'))).toContain('100')
  await scrollTo(page, 1e6)
  await expect.poll(() => requests.map(r => r.get('offset'))).toContain('200')
  await scrollTo(page, 1e6)
  await expect(grid.getByRole('link', { name: 'アルバム 249' })).toBeVisible()
})

test('詳細から「戻る」と読み直さずに元の位置に戻り、リンクで開き直すと先頭から読む', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  const { requests } = await mockLibrary(page)
  await page.goto('/library/albums')
  await expect(page.getByRole('link', { name: 'アルバム 000' })).toBeVisible()

  await scrollTo(page, 1500)
  await expect.poll(() => scrollTop(page)).toBeGreaterThan(1000)
  const before = await scrollTop(page)
  const link = page.getByTestId('album-grid').getByRole('link').nth(4)
  const name = (await link.locator('span span').first().textContent())!
  // click() は押す前にリンクまでスクロールするので、位置を変えずに押す
  await link.dispatchEvent('click')

  await expect(page.getByRole('heading', { name })).toBeVisible()
  await expect(page.getByTestId('songs').getByRole('listitem')).toHaveCount(2)
  await expect(page.getByText('ゲスト')).toBeVisible()
  await expect.poll(() => scrollTop(page)).toBe(0)
  const loaded = requests.length

  await page.getByRole('button', { name: '戻る', exact: true }).click()
  await expect(page).toHaveURL('/library/albums')
  await expect.poll(() => scrollTop(page)).toBe(before)
  expect(requests.length).toBe(loaded)

  // 詳細からリンク（ライブラリの一覧）で開き直すと、先頭から読み直す
  await page.getByTestId('album-grid').getByRole('link').first().click()
  await expect(page.getByTestId('songs')).toBeVisible()
  await page.goto('/library')
  await page.getByRole('main').getByRole('link', { name: 'アルバム' }).click()
  await expect(page.getByRole('link', { name: 'アルバム 000' })).toBeVisible()
  expect(await scrollTop(page)).toBe(0)
  expect(requests.at(-1)?.get('offset')).toBe('0')
})

test('直に開いたアルバムの詳細の「戻る」は、アルバムの一覧に移る', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3 })
  await page.goto('/library/albums/al-1')
  await expect(page.getByRole('heading', { name: 'アルバム 001' })).toBeVisible()

  await page.getByRole('button', { name: '戻る', exact: true }).click()
  await expect(page).toHaveURL('/library/albums')
  await expect(page.getByRole('link', { name: 'アルバム 000' })).toBeVisible()
})

test('表示の切り替えと並び順は開発モードのときだけ出す', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3 })
  await page.goto('/library/albums')
  await expect(page.getByRole('link', { name: 'アルバム 000' })).toBeVisible()
  await expect(page.getByTestId('album-controls')).toHaveCount(0)
})

test('開発モードでは未実装の表示と並び順を押せない状態で出す', async ({ page }) => {
  await mockApi(page, { loggedIn: true, dev: true })
  await mockLibrary(page, { count: 3 })
  await page.goto('/library/albums')
  await expect(page.getByRole('radio', { name: 'グリッド' })).toHaveAttribute('aria-checked', 'true')
  await expect(page.getByRole('radio', { name: 'リスト' })).toBeDisabled()
  await expect(page.getByRole('combobox', { name: '並び順' })).toHaveValue('name')
  await expect(page.getByRole('option', { name: '新着順' })).toBeDisabled()
})

test('セッションが切れていればログインの画面に移す', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await page.route(/\/rest\/getAlbumList2/, route => route.fulfill({
    json: { 'subsonic-response': { status: 'failed', error: { code: 40, message: 'Wrong username or password' } } },
  }))
  await page.goto('/library/albums')
  await expect(page).toHaveURL(/\/login\?redirect=(%2F|\/)library(%2F|\/)albums$/)
})

test('スマホは一覧の見出しの帯にライブラリへ戻る矢印を出し、PC は出さない', async ({ page, isMobile }) => {
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3 })
  await page.goto('/library/albums')
  const back = page.getByRole('button', { name: 'ライブラリに戻る' })
  if (!isMobile) {
    await expect(back).toBeHidden()
    return
  }
  await back.click()
  await expect(page).toHaveURL('/library')
})

test('見出しの帯はスクロールしても残り、詳細では大きな見出しが隠れたら名前を出す', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 30 })
  await page.goto('/library/albums/al-1')
  await expect(page.getByTestId('songs').getByRole('listitem')).toHaveCount(30)
  const header = page.getByTestId('page-header')
  const title = header.getByText('アルバム 001')
  await expect(title).toHaveCSS('opacity', '0')

  await scrollTo(page, 800)
  await expect(title).toHaveCSS('opacity', '1')
  const box = (await header.boundingBox())!
  const scroller = (await page.getByTestId('scroller').boundingBox())!
  expect(Math.abs(box.y - scroller.y)).toBeLessThan(2)
})

test('長いアルバム名は、はみ出した分を…で省く', async ({ page }) => {
  const long = 'とても長いアルバムの名前'.repeat(8)
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 30, name: () => long })
  const clipped = (locator: Locator) => locator.evaluate(el =>
    el.scrollWidth > el.clientWidth || el.scrollHeight > el.clientHeight)

  await page.goto('/library/albums')
  const card = page.getByTestId('album-grid').getByText(long).first()
  await expect(card).toHaveCSS('text-overflow', 'ellipsis')
  expect(await clipped(card)).toBe(true)

  await page.goto('/library/albums/al-1')
  // 大きな見出しは 2 行まで出し、残りを…で省く
  const heading = page.getByRole('heading', { name: long })
  await expect(heading).toHaveCSS('-webkit-line-clamp', '2')
  expect(await clipped(heading)).toBe(true)
  await scrollTo(page, 1200)
  const bar = page.getByTestId('page-header').getByText(long)
  await expect(bar).toHaveCSS('opacity', '1')
  await expect(bar).toHaveCSS('text-overflow', 'ellipsis')
  expect(await clipped(bar)).toBe(true)
})
