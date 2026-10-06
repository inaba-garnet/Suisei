import { expect, test } from '@playwright/test'
import { mockApi, mockScan } from './api'

test.beforeEach(async ({ page }) => {
  await mockApi(page, { loggedIn: true })
})

test('今すぐスキャンで始め、終わるまで進みを出す', async ({ page }) => {
  const scan = await mockScan(page)
  await page.goto('/settings')
  const button = page.getByRole('button', { name: '今すぐスキャン' })
  await button.click()
  await expect(button).toBeDisabled()
  await expect(page.getByTestId('scan-status')).toContainText('スキャン中')
  await expect(page.getByRole('status')).toHaveText('スキャンが終わりました')
  await expect(button).toBeEnabled()
  expect(scan.starts).toHaveLength(1)
  expect(scan.starts[0]!.get('fullScan')).toBeNull()
})

test('定期のスキャンが走っていれば、開いたときから進みを出す', async ({ page }) => {
  await mockScan(page, { running: true, polls: 3 })
  await page.goto('/settings')
  await expect(page.getByRole('button', { name: '今すぐスキャン' })).toBeDisabled()
  await expect(page.getByTestId('scan-status')).toContainText('件')
  await expect(page.getByRole('button', { name: '今すぐスキャン' })).toBeEnabled()
})

test('全ファイルを読み直すスキャンは詳細から始める', async ({ page }) => {
  const scan = await mockScan(page)
  await page.goto('/settings')
  await page.getByText('詳細', { exact: true }).click()
  await page.getByRole('button', { name: '全ファイルを読み直す' }).click()
  await expect(page.getByRole('status')).toHaveText('スキャンが終わりました')
  expect(scan.starts[0]!.get('fullScan')).toBe('true')
})

test('最後のスキャンの結果と、読めなかったファイルを出す', async ({ page }) => {
  await mockScan(page, {
    last: { at: Date.now() - 5 * 60_000, full: false, elapsedMs: 1200, files: 3066, read: 12, failed: 2, failedPaths: ['a/壊れた.flac'] },
  })
  await page.goto('/settings')
  await expect(page.getByTestId('scan-status')).toHaveText('5 分前のスキャン: 3,066 ファイル、読み直し 12 件')
  const failed = page.getByTestId('scan-failed')
  await expect(failed.getByText('a/壊れた.flac')).toBeHidden()
  await failed.getByText('読めなかったファイル 2 件').click()
  await expect(failed.getByText('a/壊れた.flac')).toBeVisible()
  await expect(failed.getByText('ほか 1 件')).toBeVisible()
})

test('失敗したスキャンは理由を出す', async ({ page }) => {
  await mockScan(page, { result: { error: '音楽フォルダを読めない', files: undefined } })
  await page.goto('/settings')
  await page.getByRole('button', { name: '今すぐスキャン' }).click()
  await expect(page.getByRole('status')).toHaveText('スキャンに失敗しました')
  await expect(page.getByTestId('scan-status')).toContainText('失敗しました（音楽フォルダを読めない）')
})
