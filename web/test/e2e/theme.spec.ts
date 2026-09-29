import { expect, test } from '@playwright/test'
import { mockApi } from './api'

test.beforeEach(async ({ page }) => {
  await mockApi(page, { loggedIn: true })
})

test('既定では OS の設定に従う', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'light' })
  await page.goto('/')
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light')

  await page.emulateMedia({ colorScheme: 'dark' })
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark')
})

test('設定で選んだテーマを再読み込みの後も保つ', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'dark' })
  await page.goto('/settings')
  await page.getByRole('radio', { name: 'ライト' }).click()
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light')
  await expect(page.locator('meta[name="theme-color"]')).toHaveAttribute('content', '#edf0fa')

  await page.reload()
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light')
  await expect(page.getByRole('radio', { name: 'ライト' })).toHaveAttribute('aria-checked', 'true')

  await page.getByRole('radio', { name: 'OS に合わせる' }).click()
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark')
})
