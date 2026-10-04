import { expect, test } from '@playwright/test'
import { mockApi, mockLibrary, mockPlaylists, mockStream } from './api'

const menu = (page: import('@playwright/test').Page) => page.getByRole('menu')

test('曲の「…」から「次に再生」で、再生中の曲の直後に入れる', async ({ page, isMobile }) => {
  test.skip(isMobile, '次の曲のボタンは PC だけ')
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 4 })
  const { ids } = await mockStream(page)
  await page.goto('/library/albums/al-1')
  await page.getByRole('button', { name: 'すべて再生' }).click()

  await page.getByRole('button', { name: '曲 4のメニュー' }).click()
  await menu(page).getByRole('menuitem', { name: '次に再生' }).click()
  await expect(menu(page)).toHaveCount(0)
  await expect(page.getByRole('status')).toHaveText('次に再生します')

  const bar = page.getByTestId('player-bar')
  await bar.getByRole('button', { name: '次の曲' }).click()
  await expect(bar).toContainText('曲 4')
  await bar.getByRole('button', { name: '次の曲' }).click()
  await expect(bar).toContainText('二曲目')
  await expect.poll(() => ids).toEqual(['al-1-1', 'al-1-4', 'al-1-2'])
})

test('シャッフル中に「次に再生」で入れた曲は、シャッフルをオフにしても再生中の曲の次に残る', async ({ page, isMobile }) => {
  test.skip(isMobile, '右の欄は広い画面だけ')
  await page.setViewportSize({ width: 1440, height: 900 })
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 4 })
  const { ids } = await mockStream(page)
  await page.goto('/library/albums/al-1')
  await page.getByRole('button', { name: 'すべて再生' }).click()
  const panel = page.getByTestId('player-panel')
  await panel.getByRole('button', { name: 'シャッフル' }).click()

  await page.getByRole('button', { name: '曲 3のメニュー' }).click()
  await menu(page).getByRole('menuitem', { name: '次に再生' }).click()
  await panel.getByRole('button', { name: 'シャッフル' }).click()
  await panel.getByRole('button', { name: '次の曲' }).click()
  await expect.poll(() => ids.at(-1)).toBe('al-1-3')
})

test('何も鳴らしていなければ、「キューの最後に追加」でその曲から鳴らし始める', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 3 })
  const { ids } = await mockStream(page)
  await page.goto('/library/albums/al-1')
  await page.getByRole('button', { name: '二曲目のメニュー' }).click()
  await menu(page).getByRole('menuitem', { name: 'キューの最後に追加' }).click()
  await expect(page.getByTestId('player-bar')).toContainText('二曲目')
  await expect.poll(() => ids).toEqual(['al-1-2'])
})

test('アルバムの「…」はアルバムの全曲を入れ、アルバムの詳細では「アルバムへ移動」を出さない', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 3 })
  const { ids } = await mockStream(page)
  await page.goto('/library/albums/al-1')
  await page.getByRole('button', { name: 'アルバムのメニュー' }).click()
  await expect(menu(page).getByRole('menuitem', { name: 'アルバムへ移動' })).toHaveCount(0)
  await menu(page).getByRole('menuitem', { name: '次に再生' }).click()
  await expect(page.getByTestId('player-bar')).toContainText('一曲目')
  await expect.poll(() => ids).toEqual(['al-1-1'])

  // 曲のメニューでも、開いているアルバムには移らない
  await page.getByRole('button', { name: '一曲目のメニュー' }).click()
  await expect(menu(page).getByRole('menuitem', { name: 'アルバムへ移動' })).toHaveCount(0)
  await menu(page).getByRole('menuitem', { name: 'アーティストへ移動' }).click()
  await expect(page).toHaveURL('/library/artists/ar-1?view=tracks')
})

test('既存のプレイリストに足し、新しいプレイリストを作って足す', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 3 })
  const { playlists, updates } = await mockPlaylists(page, { count: 1, songs: 0 })
  const creates: URLSearchParams[] = []
  page.on('request', (request) => {
    if (request.url().includes('/rest/createPlaylist')) {
      creates.push(new URLSearchParams(request.postData() ?? ''))
    }
  })
  await page.goto('/library/albums/al-1')

  await page.getByRole('button', { name: '二曲目のメニュー' }).click()
  await menu(page).getByRole('menuitem', { name: 'プレイリストに追加' }).click()
  await menu(page).getByRole('menuitem', { name: 'プレイリスト 0' }).click()
  await expect(page.getByRole('status')).toHaveText('「プレイリスト 0」に追加しました')
  expect(updates[0]?.getAll('songIdToAdd')).toEqual(['al-1-2'])
  expect(playlists[0]?.entry.map(s => s.id)).toEqual(['al-1-2'])

  await page.getByRole('button', { name: 'アルバムのメニュー' }).click()
  await menu(page).getByRole('menuitem', { name: 'プレイリストに追加' }).click()
  await menu(page).getByRole('menuitem', { name: '新規プレイリスト' }).click()
  await menu(page).getByRole('textbox', { name: '新しいプレイリストの名前' }).fill('お出かけ')
  await menu(page).getByRole('button', { name: '作成' }).click()
  await expect(page.getByRole('status')).toHaveText('「お出かけ」を作って追加しました')
  expect(creates[0]?.get('name')).toBe('お出かけ')
  expect(creates[0]?.getAll('songId')).toEqual(['al-1-1', 'al-1-2', 'al-1-3'])
})

test('曲の「…」から曲のアーティストと、曲のアルバムに移る', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 3 })
  await page.goto('/library/albums/al-2')
  // 曲のアーティストが二人以上なら名前を添え、一人なら「アーティストへ移動」
  await page.getByRole('button', { name: '二曲目のメニュー' }).click()
  await menu(page).getByRole('menuitem', { name: 'アーティストへ移動' }).click()
  await expect(page).toHaveURL('/library/artists/ar-guest?view=tracks')
  await page.goBack()
  await page.getByRole('button', { name: '一曲目のメニュー' }).click()
  await menu(page).getByRole('menuitem', { name: 'アーティストへ移動' }).click()
  await expect(page).toHaveURL('/library/artists/ar-1?view=tracks')
})

test('Esc で閉じて「…」にフォーカスを戻し、矢印で項目を移る', async ({ page, isMobile }) => {
  test.skip(isMobile, 'キーボードの操作')
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 3 })
  await page.goto('/library/albums/al-1')
  const button = page.getByRole('button', { name: '二曲目のメニュー' })
  await button.click()
  await expect(menu(page).getByRole('menuitem', { name: '次に再生' })).toBeFocused()
  await page.keyboard.press('ArrowDown')
  await expect(menu(page).getByRole('menuitem', { name: 'キューの最後に追加' })).toBeFocused()
  await page.keyboard.press('ArrowUp')
  await page.keyboard.press('ArrowUp')
  await expect(menu(page).getByRole('menuitem').last()).toBeFocused()
  await page.keyboard.press('Escape')
  await expect(menu(page)).toHaveCount(0)
  await expect(button).toBeFocused()
})

test('PC はボタンの横に出し、スマホは下からシートで出して何の操作かを上に出す', async ({ page, isMobile }) => {
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 3 })
  await page.goto('/library/albums/al-1')
  const button = page.getByRole('button', { name: '二曲目のメニュー' })
  await button.click()
  const box = (await menu(page).boundingBox())!
  const viewport = page.viewportSize()!
  if (isMobile) {
    expect(Math.abs(box.y + box.height - viewport.height)).toBeLessThan(2)
    expect(box.width).toBeGreaterThan(viewport.width - 2)
    await expect(menu(page)).toContainText('二曲目')
  }
  else {
    const anchor = (await button.boundingBox())!
    expect(Math.abs(box.x + box.width - (anchor.x + anchor.width))).toBeLessThan(2)
    expect(box.width).toBeLessThan(300)
  }
  // 後ろを押すと閉じる
  await page.mouse.click(5, 5)
  await expect(menu(page)).toHaveCount(0)
})
