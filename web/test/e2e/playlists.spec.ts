import type { Locator, Page } from '@playwright/test'
import { expect, test } from '@playwright/test'
import { mockApi, mockPlaylists, mockStream } from './api'

async function touchDrag(page: Page, x: number, y: number, dx: number, dy: number) {
  const cdp = await page.context().newCDPSession(page)
  await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ x, y }] })
  for (let i = 1; i <= 10; i++) {
    await cdp.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [{ x: x + (dx * i) / 10, y: y + (dy * i) / 10 }] })
  }
  await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] })
  await cdp.detach()
}

async function center(locator: Locator) {
  const box = (await locator.boundingBox())!
  return { x: box.x + box.width / 2, y: box.y + box.height / 2 }
}

const editTitles = (page: Page) => page.getByTestId('playlist-edit-title').allTextContents()

test('一覧から開いて曲を並べ、「すべて再生」で一曲目から鳴らす', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockPlaylists(page)
  const { ids } = await mockStream(page)
  await page.goto('/playlists')

  const list = page.getByTestId('playlists')
  await expect(list.getByRole('listitem')).toHaveCount(2)
  await expect(list.getByRole('listitem').first()).toContainText('4 曲 · 13:20')
  await list.getByRole('link', { name: /プレイリスト 1/ }).click()

  await expect(page).toHaveURL('/playlists/pl-1')
  await expect(page.getByRole('heading', { name: 'プレイリスト 1' })).toBeVisible()
  await expect(page.getByTestId('songs').getByRole('listitem')).toHaveCount(4)
  await page.getByRole('button', { name: 'すべて再生' }).click()
  await expect(page.getByTestId('player-bar')).toContainText('曲 0')
  await expect.poll(() => ids).toEqual(['tr-0'])

  await page.getByRole('button', { name: '戻る', exact: true }).click()
  await expect(page).toHaveURL('/playlists')
})

test('名前を入れて作り、作ったプレイリストを開く', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockPlaylists(page, { count: 0 })
  await page.goto('/playlists')
  await expect(page.getByText('プレイリストがありません')).toBeVisible()

  await page.getByRole('button', { name: '新規プレイリスト' }).click()
  const create = page.getByRole('button', { name: '作成' })
  await expect(create).toBeDisabled()
  await page.getByRole('textbox', { name: '新しいプレイリストの名前' }).fill('ドライブ')
  await create.click()
  await expect(page).toHaveURL('/playlists/pl-0')
  await expect(page.getByRole('heading', { name: 'ドライブ' })).toBeVisible()
  await expect(page.getByText('曲がありません')).toBeVisible()

  await page.goto('/playlists')
  await expect(page.getByTestId('playlists').getByRole('listitem')).toHaveCount(1)
})

test('PC では編集で名前を変え、× で外し、取っ手で並べ替えて「完了」でまとめて保存する', async ({ page, isMobile }) => {
  test.skip(isMobile, 'PC の操作')
  await mockApi(page, { loggedIn: true })
  const { updates } = await mockPlaylists(page)
  await page.goto('/playlists/pl-0')
  await page.getByRole('button', { name: '編集' }).click()

  await page.getByRole('textbox', { name: 'プレイリストの名前' }).fill('新しい名前')
  await page.getByRole('button', { name: '曲 1をプレイリストから外す' }).click()
  await expect.poll(() => editTitles(page)).toEqual(['曲 0', '曲 2', '曲 3'])

  const rows = page.getByTestId('playlist-edit').locator('li')
  const from = await center(rows.nth(2).locator('[data-playlist-handle]'))
  const to = await center(rows.nth(0).locator('[data-playlist-handle]'))
  await page.mouse.move(from.x, from.y)
  await page.mouse.down()
  await page.mouse.move(to.x, to.y - 10, { steps: 10 })
  await page.mouse.up()
  await expect.poll(() => editTitles(page)).toEqual(['曲 3', '曲 0', '曲 2'])
  // 「完了」までは保存しない
  expect(updates).toHaveLength(0)

  await page.getByRole('button', { name: '完了' }).click()
  await expect(page.getByRole('heading', { name: '新しい名前' })).toBeVisible()
  expect(updates).toHaveLength(1)
  expect(updates[0]?.get('name')).toBe('新しい名前')
  expect(updates[0]?.getAll('songIndexToRemove')).toEqual(['0', '1', '2', '3'])
  expect(updates[0]?.getAll('songIdToAdd')).toEqual(['tr-3', 'tr-0', 'tr-2'])
  const songs = page.getByTestId('songs')
  await expect(songs.getByRole('listitem')).toHaveCount(3)
  await expect(songs.getByRole('listitem').first()).toContainText('曲 3')
})

test('「キャンセル」なら編集を捨てて保存しない', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  const { updates } = await mockPlaylists(page)
  await page.goto('/playlists/pl-0')
  await page.getByRole('button', { name: '編集' }).click()
  await page.getByRole('textbox', { name: 'プレイリストの名前' }).fill('捨てる名前')
  await page.getByRole('button', { name: 'キャンセル' }).click()
  await expect(page.getByRole('heading', { name: 'プレイリスト 0' })).toBeVisible()
  await expect(page.getByTestId('songs').getByRole('listitem')).toHaveCount(4)
  expect(updates).toHaveLength(0)
})

test('スマホでは編集中の行を左に払って外し、取っ手を指で引いて並べ替える', async ({ page, isMobile }) => {
  test.skip(!isMobile, 'スマホの操作')
  await mockApi(page, { loggedIn: true })
  const { updates } = await mockPlaylists(page)
  await page.goto('/playlists/pl-0')
  await page.getByRole('button', { name: '編集' }).click()
  await expect(page.getByRole('button', { name: /をプレイリストから外す$/ })).toHaveCount(0)

  const second = await center(page.getByTestId('playlist-edit').getByText('曲 1'))
  await touchDrag(page, second.x, second.y, -200, 0)
  await expect.poll(() => editTitles(page)).toEqual(['曲 0', '曲 2', '曲 3'])

  const rows = page.getByTestId('playlist-edit').locator('li')
  const from = await center(rows.nth(1).locator('[data-playlist-handle]'))
  const to = await center(rows.nth(0).locator('[data-playlist-handle]'))
  await touchDrag(page, from.x, from.y, 0, to.y - from.y - 10)
  await expect.poll(() => editTitles(page)).toEqual(['曲 2', '曲 0', '曲 3'])

  await page.getByRole('button', { name: '完了' }).click()
  await expect(page.getByTestId('songs')).toBeVisible()
  expect(updates[0]?.getAll('songIdToAdd')).toEqual(['tr-2', 'tr-0', 'tr-3'])
})

test('保存に失敗したら編集を続けたまま知らせる', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockPlaylists(page)
  await page.route(/\/rest\/updatePlaylist/, route => route.fulfill({ status: 500 }))
  await page.goto('/playlists/pl-0')
  await page.getByRole('button', { name: '編集' }).click()
  await page.getByRole('textbox', { name: 'プレイリストの名前' }).fill('新しい名前')
  await page.getByRole('button', { name: '完了' }).click()
  await expect(page.getByText('保存できませんでした')).toBeVisible()
  await expect(page.getByRole('textbox', { name: 'プレイリストの名前' })).toHaveValue('新しい名前')
})

test('編集から確かめてプレイリストを削除し、一覧に戻る', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockPlaylists(page)
  await page.goto('/playlists')
  await page.getByTestId('playlists').getByRole('link', { name: /プレイリスト 0/ }).click()
  await page.getByRole('button', { name: '編集' }).click()
  await page.getByRole('button', { name: 'プレイリストを削除' }).click()
  await expect(page.getByText('「プレイリスト 0」を削除しますか？')).toBeVisible()
  await page.getByRole('button', { name: 'やめる' }).click()
  await page.getByRole('button', { name: 'プレイリストを削除' }).click()
  await page.getByRole('button', { name: '削除する' }).click()

  await expect(page).toHaveURL('/playlists')
  await expect(page.getByTestId('playlists').getByRole('listitem')).toHaveCount(1)
  await expect(page.getByTestId('playlists')).not.toContainText('プレイリスト 0')
})
