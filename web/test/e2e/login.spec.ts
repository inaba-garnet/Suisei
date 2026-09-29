import { expect, test } from '@playwright/test'
import { mockApi, PASSWORD, USER } from './api'

test('ログインしていなければログインの画面に移し、ログインの後は元の画面に戻す', async ({ page }) => {
  await mockApi(page)
  await page.goto('/library/albums')
  await expect(page).toHaveURL('/login?redirect=/library/albums')

  await page.getByLabel('利用者名').fill(USER)
  await page.getByLabel('パスワード').fill('wrong')
  await page.getByRole('button', { name: 'ログイン' }).click()
  await expect(page.getByRole('alert')).toHaveText('利用者名かパスワードが違います')

  await page.getByLabel('パスワード').fill(PASSWORD)
  await page.getByRole('button', { name: 'ログイン' }).click()
  await expect(page).toHaveURL('/library/albums')
  await expect(page.getByRole('heading', { name: 'アルバム' })).toBeVisible()
})

test('別のサイトへの戻り先は無視してホームに移る', async ({ page }) => {
  await mockApi(page)
  await page.goto('/login?redirect=//example.com/')
  await page.getByLabel('利用者名').fill(USER)
  await page.getByLabel('パスワード').fill(PASSWORD)
  await page.getByRole('button', { name: 'ログイン' }).click()
  await expect(page).toHaveURL('/')
})

test('ログイン中にログインの画面を開くとホームに移る', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await page.goto('/login')
  await expect(page).toHaveURL('/')
})

test('サーバーに届かなければそう伝える', async ({ page }) => {
  await mockApi(page)
  await page.goto('/login')
  await page.route('**/api/login', route => route.fulfill({ status: 502 }))
  await page.getByLabel('利用者名').fill(USER)
  await page.getByLabel('パスワード').fill(PASSWORD)
  await page.getByRole('button', { name: 'ログイン' }).click()
  await expect(page.getByRole('alert')).toHaveText('サーバーに接続できません')
})

test('ログアウトするとログインの画面に戻る', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await page.goto('/settings')
  await expect(page.getByTestId('username')).toHaveText(USER)
  await page.getByRole('button', { name: 'ログアウト' }).click()
  await expect(page).toHaveURL('/login')
  await page.goto('/')
  await expect(page).toHaveURL('/login?redirect=/')
})
