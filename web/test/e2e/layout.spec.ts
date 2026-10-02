import { expect, test } from '@playwright/test'
import { mockApi } from './api'

test('PC は開発モードのとき内容の配置を中央と左寄せから選べ、再読み込みの後も保つ', async ({ page, isMobile }) => {
  test.skip(isMobile, 'PC の画面だけ')
  await mockApi(page, { loggedIn: true, dev: true })
  // 内容の幅が 640px までの画面で、左右に余りが出るようにする
  await page.setViewportSize({ width: 1100, height: 800 })
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

test('タブとホーム画面のアイコンを配る', async ({ page, request }) => {
  await mockApi(page, { loggedIn: true })
  await page.goto('/')
  for (const selector of ['link[rel="icon"][sizes="32x32"]', 'link[rel="icon"][sizes="192x192"]', 'link[rel="apple-touch-icon"]']) {
    const href = await page.locator(selector).getAttribute('href')
    const res = await request.get(href!)
    expect(res.status(), selector).toBe(200)
    expect(res.headers()['content-type'], selector).toContain('image/png')
  }
})

test('画面の幅に合わせて内容の幅を広げ、広い画面では右の欄に再生プレイヤーを置く', async ({ page, isMobile }) => {
  test.skip(isMobile, 'PC の画面だけ')
  await mockApi(page, { loggedIn: true })
  await page.goto('/settings')
  const content = page.getByTestId('content')
  const panel = page.getByTestId('player-panel')
  const bar = page.getByTestId('player-bar')
  const width = async () => Math.round((await content.boundingBox())!.width)

  for (const [viewport, contentWidth, hasPanel, panelWidth] of [
    [1100, 640, false, 0],
    [1300, 1040, false, 0],
    [1500, 1040, true, 320],
    [1800, 1040, true, 380],
    [2560, 1440, true, 400],
  ] as const) {
    await page.setViewportSize({ width: viewport, height: 900 })
    await expect.poll(width, String(viewport)).toBeLessThanOrEqual(contentWidth)
    if (hasPanel) {
      await expect(panel).toBeVisible()
      await expect(bar).toBeHidden()
      // 欄の幅から、外側の余白（右 20px）を除いた幅
      await expect.poll(async () => Math.round((await panel.boundingBox())!.width) + 20, String(viewport)).toBe(panelWidth)
    }
    else {
      await expect(panel).toBeHidden()
      await expect(bar).toBeVisible()
    }
  }
})
