import { expect, test } from '@playwright/test'
import { mockApi } from './api'

const TABS = ['ホーム', '検索', 'ライブラリ', 'プレイリスト', '設定']
const LIBRARY = ['アルバム', 'アーティスト', 'トラック', 'ジャンル', 'お気に入り']

test.beforeEach(async ({ page }) => {
  await mockApi(page, { loggedIn: true })
})

test('スマホは下のタブで移り、ライブラリは一覧から項目に入る', async ({ page, isMobile }) => {
  test.skip(!isMobile, 'スマホの画面だけ')
  await page.goto('/')
  const tabs = page.getByRole('navigation', { name: 'メイン' })
  await expect(tabs).toHaveCount(1)
  await expect(tabs.getByRole('link')).toHaveText(TABS)
  await expect(page.getByTestId('player-bar')).toBeVisible()

  await tabs.getByRole('link', { name: 'ライブラリ' }).click()
  await expect(page).toHaveURL('/library')
  await expect(page.getByRole('main').getByRole('link')).toHaveText(LIBRARY)

  await page.getByRole('main').getByRole('link', { name: 'アルバム' }).click()
  await expect(page).toHaveURL('/library/albums')
  await expect(tabs.getByRole('link', { name: 'ライブラリ' })).toHaveAttribute('aria-current', 'page')

  // 表示中のタブをもう一度押すと、そのタブの最初の画面に戻る
  await tabs.getByRole('link', { name: 'ライブラリ' }).click()
  await expect(page).toHaveURL('/library')
})

test('PC はサイドバーにライブラリの項目を展開して出す', async ({ page, isMobile }) => {
  test.skip(isMobile, 'PC の画面だけ')
  await page.goto('/')
  const sidebar = page.getByRole('navigation', { name: 'メイン' })
  await expect(sidebar).toHaveCount(1)
  await expect(sidebar.getByRole('link').filter({ hasNotText: 'Suisei' })).toHaveText([
    'ホーム', '検索', 'ライブラリ', ...LIBRARY, 'プレイリスト', '設定',
  ])
  await expect(page.getByTestId('player-bar')).toBeVisible()

  await sidebar.getByRole('link', { name: 'トラック' }).click()
  await expect(page).toHaveURL('/library/tracks')
  await expect(page.getByRole('heading', { name: 'トラック' })).toBeVisible()
})
