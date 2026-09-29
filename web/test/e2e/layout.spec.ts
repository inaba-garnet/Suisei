import { expect, test } from '@playwright/test'
import { mockApi } from './api'

test.beforeEach(async ({ page }) => {
  await mockApi(page, { loggedIn: true })
})

test('PC は内容の配置を中央と左寄せから選べ、再読み込みの後も保つ', async ({ page, isMobile }) => {
  test.skip(isMobile, 'PC の画面だけ')
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
