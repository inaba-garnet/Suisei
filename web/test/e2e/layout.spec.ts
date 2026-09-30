import { expect, test } from '@playwright/test'
import { mockApi } from './api'

test('PC は開発モードのとき内容の配置を中央と左寄せから選べ、再読み込みの後も保つ', async ({ page, isMobile }) => {
  test.skip(isMobile, 'PC の画面だけ')
  await mockApi(page, { loggedIn: true, dev: true })
  await page.goto('/settings')
  const content = page.getByTestId('content')
  const main = page.getByRole('main')
  const left = async () => (await content.boundingBox())!.x - (await main.boundingBox())!.x

  // 既定は中央
  await expect(page.getByRole('radio', { name: '中央' })).toHaveAttribute('aria-checked', 'true')
  const centered = await left()

  await page.getByRole('radio', { name: '左寄せ' }).click()
  await expect.poll(left).toBeLessThan(centered)
  const aligned = await left()

  await page.reload()
  await expect(page.getByRole('radio', { name: '左寄せ' })).toHaveAttribute('aria-checked', 'true')
  await expect.poll(left).toBe(aligned)

  await page.getByRole('radio', { name: '中央' }).click()
  await expect.poll(left).toBe(centered)
})

test('開発モードでなければ「開発」の項目を出さず、保存した左寄せも使わない', async ({ page, isMobile }) => {
  test.skip(isMobile, 'PC の画面だけ')
  await mockApi(page, { loggedIn: true })
  await page.addInitScript(() => localStorage.setItem('suisei-dev-content-align', 'left'))
  await page.goto('/settings')
  await expect(page.getByRole('heading', { name: 'テーマ' })).toBeVisible()
  await expect(page.getByRole('heading', { name: '開発' })).toHaveCount(0)

  const content = (await page.getByTestId('content').boundingBox())!
  const main = (await page.getByRole('main').boundingBox())!
  // 中央に置けば、左右の余白がほぼ等しい
  const left = content.x - main.x
  const right = main.x + main.width - (content.x + content.width)
  expect(Math.abs(left - right)).toBeLessThan(2)
})
