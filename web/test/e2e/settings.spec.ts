import { expect, test } from '@playwright/test'
import { mockApi, mockSettings } from './api'

test.beforeEach(async ({ page }) => {
  await mockApi(page, { loggedIn: true })
})

test('スキャンの間隔を選ぶとすぐ保存する', async ({ page }) => {
  const { saved } = await mockSettings(page)
  await page.goto('/settings')
  const select = page.getByLabel('スキャンの間隔')
  await expect(select).toHaveValue('3600')
  await select.selectOption({ label: '30 分' })
  await expect(page.getByRole('status')).toHaveText('保存しました')
  expect(saved).toEqual([expect.objectContaining({ scanInterval: 1800, backupInterval: 86400 })])

  await page.reload()
  await expect(page.getByLabel('スキャンの間隔')).toHaveValue('1800')
})

test('候補にない値も選べる形で出し、バックアップを止めると残す数は選べない', async ({ page }) => {
  await mockSettings(page, { scanInterval: 5400 })
  await page.goto('/settings')
  await expect(page.getByLabel('スキャンの間隔').locator('option:checked')).toHaveText('90 分')

  await page.getByLabel('データベースのバックアップの間隔').selectOption({ label: '止める' })
  await expect(page.getByLabel('残す数')).toBeDisabled()
})

test('保存に失敗したら元の値に戻す', async ({ page }) => {
  await mockSettings(page, {}, { fail: true })
  await page.goto('/settings')
  await page.getByLabel('スキャンの間隔').selectOption({ label: '1 日' })
  await expect(page.getByRole('status')).toHaveText('保存できませんでした')
  await expect(page.getByLabel('スキャンの間隔')).toHaveValue('3600')
})

test('CV の分け方は詳細に閉じておく', async ({ page }) => {
  const { saved } = await mockSettings(page)
  await page.goto('/settings')
  const control = page.getByRole('radiogroup', { name: 'キャラクターと声優の分け方' })
  await expect(control).toBeHidden()

  await page.getByText('詳細', { exact: true }).click()
  await expect(page.getByText('お気に入りや評価が別のアーティストに移る')).toBeVisible()
  await control.getByRole('radio', { name: '分けない' }).click()
  await expect(page.getByRole('status')).toHaveText('保存しました')
  expect(saved.at(-1)?.splitCharacters).toBe(false)
})
