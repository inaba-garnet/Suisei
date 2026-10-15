import { expect, test } from '@playwright/test'
import { mockApi, mockArtists, mockLibrary, mockReadings, mockStream } from './api'

const menu = (page: import('@playwright/test').Page) => page.getByRole('menu')

test('アルバムの「…」から読みを直して保存し、手動の読みを自動の読みに戻す', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3 })
  const { requests } = await mockReadings(page)
  await page.goto('/library/albums/al-1')

  await page.getByRole('button', { name: 'アルバムのメニュー' }).click()
  await menu(page).getByRole('menuitem', { name: '読みを編集' }).click()
  await expect(menu(page)).toHaveCount(0)
  const dialog = page.getByRole('dialog', { name: '読みを編集' })
  await expect(dialog).toContainText('アルバム 001')
  await expect(dialog.getByTestId('reading-current')).toHaveText('ヨミ（推定）')
  await expect(dialog.getByRole('button', { name: '自動の読みに戻す' })).toHaveCount(0)

  const input = dialog.getByRole('textbox', { name: '新しい読み' })
  await expect(input).toHaveValue('ヨミ')
  await input.fill('あるばむ')
  await dialog.getByRole('button', { name: '保存' }).click()
  await expect(dialog).toHaveCount(0)
  await expect(page.getByRole('status')).toHaveText('読みを保存しました')
  expect(requests).toEqual([{ id: 'al-1', reading: 'あるばむ' }])

  // 手動の読みなら自動の読みを添え、戻せる
  await page.getByRole('button', { name: 'アルバムのメニュー' }).click()
  await menu(page).getByRole('menuitem', { name: '読みを編集' }).click()
  await expect(dialog.getByTestId('reading-current')).toHaveText('あるばむ（手動）')
  await expect(dialog.getByTestId('reading-auto')).toHaveText('ヨミ（推定）')
  await dialog.getByRole('button', { name: '自動の読みに戻す' }).click()
  await expect(dialog).toHaveCount(0)
  await expect(page.getByRole('status')).toHaveText('自動の読みに戻しました')
  expect(requests.at(-1)).toEqual({ id: 'al-1', reading: null })
})

test('曲の「…」から開き、漢字を含む読みは理由を出して閉じない。キャンセルで閉じる', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3 })
  const { requests } = await mockReadings(page, { invalid: true })
  await page.goto('/library/albums/al-1')

  await page.getByRole('button', { name: '一曲目のメニュー' }).click()
  await menu(page).getByRole('menuitem', { name: '読みを編集' }).click()
  const dialog = page.getByRole('dialog', { name: '読みを編集' })
  await expect(dialog).toContainText('一曲目')
  await dialog.getByRole('textbox', { name: '新しい読み' }).fill('一きょくめ')
  await dialog.getByRole('button', { name: '保存' }).click()
  await expect(dialog.getByRole('alert')).toContainText('漢字は使えません')
  expect(requests).toEqual([{ id: 'al-1-1', reading: '一きょくめ' }])

  // 空なら送らない
  await dialog.getByRole('textbox', { name: '新しい読み' }).fill(' ')
  await dialog.getByRole('button', { name: '保存' }).click()
  await expect(dialog.getByRole('alert')).toHaveText('読みを入力してください')
  expect(requests).toHaveLength(1)

  await dialog.getByRole('button', { name: 'キャンセル' }).click()
  await expect(dialog).toHaveCount(0)
})

test('アーティストの詳細の「すべて再生」は並べたアルバムの順に鳴らし、「…」から読みを直せる', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  const { songs } = await mockArtists(page)
  const { ids } = await mockStream(page)
  await mockReadings(page)
  await page.goto('/library/artists/ar-ア-0?view=albums')

  await page.getByRole('button', { name: 'すべて再生' }).click()
  // アルバムの表示は、押してからアルバムアーティストとしての曲を読む。客演の曲は入れない
  await expect(page.getByTestId('player-bar')).toContainText('一曲目')
  await expect.poll(() => ids).toEqual(['s-1'])
  expect(songs).toEqual(['', 'albumartist'])

  await page.getByRole('button', { name: 'アーティストのメニュー' }).click()
  await expect(menu(page).getByRole('menuitem', { name: 'アーティストへ移動' })).toHaveCount(0)
  await menu(page).getByRole('menuitem', { name: 'キューの最後に追加' }).click()
  await expect(page.getByRole('status')).toHaveText('キューに追加しました')

  await page.getByRole('button', { name: 'アーティストのメニュー' }).click()
  await menu(page).getByRole('menuitem', { name: '読みを編集' }).click()
  await expect(page.getByRole('dialog', { name: '読みを編集' })).toContainText('アのアーティスト 0')
})
