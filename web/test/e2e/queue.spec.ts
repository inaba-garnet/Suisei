import type { Locator, Page } from '@playwright/test'
import { expect, test } from '@playwright/test'
import { mockApi, mockLibrary, mockStream } from './api'

/** キューの曲名を上から順に返す。 */
function titles(queue: Locator) {
  return queue.getByTestId('queue-title').allTextContents()
}

/** 指で (x, y) から (dx, dy) だけ動かして離す。スマホの表示では、マウスの操作では touch の event が起きないため。 */
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

/** 「次に再生」まで送って、再生履歴を上に隠しているか。 */
function historyHidden(queue: Locator) {
  return queue.getByTestId('queue-scroller').evaluate((el) => {
    const section = el.querySelector<HTMLElement>('[data-testid="queue-upcoming-section"]')!
    return Math.abs(el.scrollTop - section.offsetTop) < 2
  })
}

test('右の欄ではジャケットの場所をキューに入れ替え、再生中の曲を上に固定し、再生履歴を隠して出す', async ({ page, isMobile }) => {
  test.skip(isMobile, 'PC の画面だけ')
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 5 })
  const { ids } = await mockStream(page)
  await page.setViewportSize({ width: 1500, height: 900 })
  await page.goto('/library/albums/al-1')

  const panel = page.getByTestId('player-panel')
  await expect(panel.getByRole('button', { name: 'キュー', exact: true })).toBeDisabled()
  await page.getByTestId('songs').getByText('曲 3').click()
  await panel.getByRole('button', { name: 'キュー', exact: true }).click()
  const queue = panel.getByTestId('player-queue')
  await expect(panel.getByRole('heading', { name: 'キュー' })).toBeVisible()
  await expect.poll(() => titles(queue)).toEqual(['曲 3', '一曲目', '二曲目', '曲 4', '曲 5'])
  await expect.poll(() => historyHidden(queue)).toBe(true)
  await expect(queue.getByTestId('queue-history').getByRole('button').first()).toHaveClass(/opacity-60/)
  // 操作の段はジャケットの表示と同じく残す
  await expect(panel.getByRole('slider', { name: '再生位置' })).toBeVisible()
  await expect(panel.getByRole('button', { name: 'シャッフル' })).toBeVisible()

  // 下にスクロールすると再生履歴が出る
  await queue.getByTestId('queue-scroller').evaluate(el => el.scrollTo(0, 0))
  await expect.poll(() => historyHidden(queue)).toBe(false)

  await queue.getByTestId('queue-upcoming').getByText('曲 5').click()
  await expect.poll(() => titles(queue)).toEqual(['曲 5', '一曲目', '二曲目', '曲 3', '曲 4'])
  await expect.poll(() => ids).toEqual(['al-1-3', 'al-1-5'])

  // もう一度押すとジャケットの表示に戻る
  await panel.getByRole('button', { name: 'キュー', exact: true }).click()
  await expect(queue).toHaveCount(0)
  await expect(panel.getByRole('heading', { name: '再生中' })).toBeVisible()
})

test('PC では × でキューから消し、次に再生する曲を取っ手で並べ替える', async ({ page, isMobile }) => {
  test.skip(isMobile, 'PC の画面だけ')
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 6 })
  const { ids } = await mockStream(page)
  await page.setViewportSize({ width: 1500, height: 1000 })
  await page.goto('/library/albums/al-1')

  await page.getByTestId('songs').getByText('二曲目').click()
  const panel = page.getByTestId('player-panel')
  await panel.getByRole('button', { name: 'キュー', exact: true }).click()
  const queue = panel.getByTestId('player-queue')
  await expect.poll(() => titles(queue)).toEqual(['二曲目', '一曲目', '曲 3', '曲 4', '曲 5', '曲 6'])
  // 再生中の曲は消せず、再生履歴は並べ替えない
  await expect(queue.getByRole('button', { name: '二曲目をキューから消す' })).toHaveCount(0)
  await expect(queue.getByTestId('queue-history').locator('[data-queue-handle]')).toHaveCount(0)

  await queue.getByText('曲 3').hover()
  await queue.getByRole('button', { name: '曲 3をキューから消す' }).click()
  await expect.poll(() => titles(queue)).toEqual(['二曲目', '一曲目', '曲 4', '曲 5', '曲 6'])

  // 曲 6 を次に再生する曲の先頭に移す
  const rows = queue.getByTestId('queue-upcoming').locator('li')
  const from = await center(rows.nth(2).locator('[data-queue-handle]'))
  const to = await center(rows.nth(0).locator('[data-queue-handle]'))
  await page.mouse.move(from.x, from.y)
  await page.mouse.down()
  await page.mouse.move(to.x, to.y - 10, { steps: 10 })
  await page.mouse.up()
  await expect.poll(() => titles(queue)).toEqual(['二曲目', '一曲目', '曲 6', '曲 4', '曲 5'])

  await panel.getByRole('button', { name: '次の曲' }).click()
  await expect(queue.getByTestId('queue-playing')).toContainText('曲 6')
  await expect.poll(() => ids).toEqual(['al-1-2', 'al-1-6'])
})

test('下端の再生バーの PC では、キューを右端に重ねて出し、× と Esc で閉じる', async ({ page, isMobile }) => {
  test.skip(isMobile, 'PC の画面だけ')
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 4 })
  await mockStream(page)
  await page.goto('/library/albums/al-1')

  await page.getByTestId('songs').getByText('曲 3').click()
  const bar = page.getByTestId('player-bar')
  await bar.getByRole('button', { name: 'キュー', exact: true }).click()
  const drawer = page.getByRole('dialog', { name: 'キュー' })
  const queue = drawer.getByTestId('player-queue')
  await expect.poll(() => titles(queue)).toEqual(['曲 3', '一曲目', '二曲目', '曲 4'])
  await expect.poll(() => historyHidden(queue)).toBe(true)
  await drawer.getByRole('button', { name: 'キューを閉じる' }).click()
  await expect(drawer).toBeHidden()

  await bar.getByRole('button', { name: 'キュー', exact: true }).click()
  await expect(drawer).toBeVisible()
  await page.keyboard.press('Escape')
  await expect(drawer).toBeHidden()
})

/** スマホで再生画面を開き、キューに切り替える。 */
async function openQueue(page: Page) {
  await page.getByTestId('player-bar').getByRole('button', { name: '再生画面を開く' }).click()
  const sheet = page.getByRole('dialog', { name: '再生画面' })
  // 引き出し終わる（上端から 16px で止まる）のを待つ
  await expect.poll(async () => (await sheet.boundingBox())?.y).toBe(16)
  await sheet.getByRole('button', { name: 'キュー', exact: true }).click()
  return { sheet, queue: sheet.getByTestId('player-queue') }
}

test('スマホでは再生画面の中をキューに切り替え、再生履歴が残っている間は引いても閉じず、操作の段を引くと閉じる', async ({ page, isMobile }) => {
  test.skip(!isMobile, 'スマホの画面だけ')
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 12 })
  await mockStream(page)
  await page.goto('/library/albums/al-1')

  await page.getByTestId('songs').getByText('曲 3').click()
  const { sheet, queue } = await openQueue(page)
  await expect.poll(() => historyHidden(queue)).toBe(true)

  // 再生履歴が上に残っている間は、下に引いてもスクロールするだけで閉じない
  const row = await center(queue.getByTestId('queue-upcoming').getByText('曲 5'))
  await touchDrag(page, row.x, row.y, 0, 300)
  await expect(sheet).toBeVisible()
  await expect.poll(() => historyHidden(queue)).toBe(false)

  // シークのバーの上では閉じない
  const seek = await center(sheet.getByRole('slider', { name: '再生位置' }))
  await touchDrag(page, seek.x, seek.y, 0, 300)
  await expect(sheet).toBeVisible()

  // 操作の段を引くと閉じる。次に開くとジャケットの表示から始める
  const controls = await center(sheet.getByRole('button', { name: '次の曲' }))
  await touchDrag(page, controls.x, controls.y, 0, 300)
  await expect(sheet).toBeHidden()
  await page.getByTestId('player-bar').getByRole('button', { name: '再生画面を開く' }).click()
  await expect(sheet.getByRole('heading', { name: '再生中' })).toBeVisible()
  await expect(sheet.getByTestId('player-queue')).toHaveCount(0)
})

test('スマホでは行を左に払ってキューから消し、取っ手を指で引いて並べ替える', async ({ page, isMobile }) => {
  test.skip(!isMobile, 'スマホの画面だけ')
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 5 })
  await mockStream(page)
  await page.goto('/library/albums/al-1')

  await page.getByTestId('songs').getByText('二曲目').click()
  const { sheet, queue } = await openQueue(page)
  // スマホでは × を出さない
  await expect(queue.getByRole('button', { name: /をキューから消す$/ })).toHaveCount(0)
  await expect.poll(() => titles(queue)).toEqual(['二曲目', '一曲目', '曲 3', '曲 4', '曲 5'])

  const upcoming = queue.getByTestId('queue-upcoming')
  const third = await center(upcoming.getByText('曲 3'))
  await touchDrag(page, third.x, third.y, -40, 0)
  await expect.poll(() => titles(queue)).toEqual(['二曲目', '一曲目', '曲 3', '曲 4', '曲 5'])
  await touchDrag(page, third.x, third.y, -200, 0)
  await expect.poll(() => titles(queue)).toEqual(['二曲目', '一曲目', '曲 4', '曲 5'])
  // 再生中の曲は払っても消さない
  const playing = await center(queue.getByTestId('queue-playing').locator('li'))
  await touchDrag(page, playing.x, playing.y, -200, 0)
  await expect.poll(() => titles(queue)).toEqual(['二曲目', '一曲目', '曲 4', '曲 5'])

  // 取っ手を引いても再生画面は閉じない
  const rows = upcoming.locator('li')
  const from = await center(rows.nth(1).locator('[data-queue-handle]'))
  const to = await center(rows.nth(0).locator('[data-queue-handle]'))
  await touchDrag(page, from.x, from.y, 0, to.y - from.y - 10)
  await expect.poll(() => titles(queue)).toEqual(['二曲目', '一曲目', '曲 5', '曲 4'])
  await expect(sheet).toBeVisible()
})
