import type { Page } from '@playwright/test'
import { expect, test } from '@playwright/test'
import { mockApi, mockLibrary, mockStream, mockTracks } from './api'

/** 指で (x, y) から `distance` だけ下に引いて離す。スマホの表示では、マウスの操作では pointer の event が起きないため。 */
async function dragDown(page: Page, x: number, y: number, distance: number) {
  const cdp = await page.context().newCDPSession(page)
  await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ x, y }] })
  for (let i = 1; i <= 8; i++) {
    await cdp.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [{ x, y: y + (distance * i) / 8 }] })
  }
  await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] })
  await cdp.detach()
}

test('アルバムの「すべて再生」で一曲目から鳴らし、終われば次の曲に進み、最後の曲で止まる', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3 })
  const { ids } = await mockStream(page, { seconds: 0.5 })
  await page.goto('/library/albums/al-1')

  const bar = page.getByTestId('player-bar')
  await expect(bar).toContainText('再生していません')
  await expect(bar.getByRole('button', { name: '再生', exact: true })).toBeDisabled()

  await page.getByRole('button', { name: 'すべて再生' }).click()
  await expect(bar).toContainText('一曲目')
  await expect.poll(() => ids).toEqual(['al-1-1', 'al-1-2'])
  await expect(bar).toContainText('二曲目')
  // 最後の曲が終わったら、その曲を出したまま止める
  await expect(bar.getByRole('button', { name: '再生', exact: true })).toBeEnabled()
  await expect(page.getByTestId('songs').getByText('二曲目')).toHaveClass(/text-accent-base/)
})

test('曲の行を押すとその曲から鳴らし、一時停止と再開ができ、画面を移っても途切れない', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockTracks(page)
  const { ids } = await mockStream(page)
  await page.goto('/library/tracks')

  await page.getByTestId('songs').getByText('title:曲 002').click()
  const bar = page.getByTestId('player-bar')
  await expect(bar).toContainText('title:曲 002')
  const pause = bar.getByRole('button', { name: '一時停止' })
  await expect(pause).toBeVisible()
  expect(ids).toEqual(['tr-002'])

  await pause.click()
  await expect(bar.getByRole('button', { name: '再生', exact: true })).toBeVisible()
  await bar.getByRole('button', { name: '再生', exact: true }).click()
  await expect(pause).toBeVisible()

  // 画面を移っても同じ曲を鳴らし続け、読み直さない
  await page.getByRole('link', { name: '設定' }).click()
  await expect(page).toHaveURL('/settings')
  await expect(bar).toContainText('title:曲 002')
  await expect(pause).toBeVisible()
  expect(ids).toEqual(['tr-002'])
})

test('前後の曲に移り、前の曲は先まで進んでいれば曲の頭に戻す', async ({ page, isMobile }) => {
  test.skip(isMobile, '前後の曲のボタンは PC だけ')
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 3 })
  const { ids } = await mockStream(page)
  await page.goto('/library/albums/al-1')

  const bar = page.getByTestId('player-bar')
  await page.getByTestId('songs').getByText('二曲目').click()
  await expect(bar).toContainText('二曲目')
  await bar.getByRole('button', { name: '次の曲' }).click()
  await expect(bar).toContainText('曲 3')
  // キューの最後なので次の曲はない
  await expect(bar.getByRole('button', { name: '次の曲' })).toBeDisabled()

  await bar.getByRole('button', { name: '前の曲' }).click()
  await expect(bar).toContainText('二曲目')
  await expect.poll(() => ids).toEqual(['al-1-2', 'al-1-3', 'al-1-2'])

  // 3 秒より先まで進んでいれば、前の曲には移らず頭に戻す
  await expect(bar).toContainText('0:04 /', { timeout: 10_000 })
  await bar.getByRole('button', { name: '前の曲' }).click()
  await expect(bar).toContainText('0:00 /')
  await expect(bar).toContainText('二曲目')
  expect(ids).toHaveLength(3)
})

test('鳴らせない曲は再生バーで知らせる', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3 })
  await mockStream(page, { fail: true })
  await page.goto('/library/albums/al-1')

  await page.getByRole('button', { name: 'すべて再生' }).click()
  const bar = page.getByTestId('player-bar')
  await expect(bar).toContainText('再生できませんでした')
  await expect(bar.getByRole('button', { name: '再生', exact: true })).toBeVisible()
})

test('広い画面では右の欄に再生中の曲を出して操作する', async ({ page, isMobile }) => {
  test.skip(isMobile, 'PC の画面だけ')
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3 })
  await mockStream(page)
  await page.setViewportSize({ width: 1500, height: 900 })
  await page.goto('/library/albums/al-1')

  const panel = page.getByTestId('player-panel')
  await expect(panel).toContainText('再生していません')
  // 何も再生していなければシャッフルは押せない
  await expect(panel.getByRole('button', { name: 'シャッフル' })).toBeDisabled()

  await page.getByRole('button', { name: 'すべて再生' }).click()
  await expect(panel).toContainText('一曲目')
  await expect(panel).toContainText('アーティスト 1')
  await panel.getByRole('button', { name: '一時停止' }).click()
  await expect(panel.getByRole('button', { name: '再生', exact: true })).toBeVisible()
  await panel.getByRole('button', { name: '次の曲' }).click()
  await expect(panel).toContainText('二曲目')
})

test('スマホでは再生バーを押すと再生画面を下から引き出し、矢印、Esc、後ろの画面、パネルを引くと閉じる', async ({ page, isMobile }) => {
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 3 })
  await mockStream(page)
  await page.goto('/library/albums/al-1')

  const bar = page.getByTestId('player-bar')
  const open = bar.getByRole('button', { name: '再生画面を開く' })
  if (!isMobile) {
    // PC では開かないので、ボタンにしない
    await expect(open).toHaveCount(0)
    return
  }
  // 何も再生していなければ開けない
  await expect(open).toBeDisabled()

  await page.getByRole('button', { name: 'すべて再生' }).click()
  await open.click()
  const sheet = page.getByRole('dialog', { name: '再生画面' })
  await expect(sheet).toContainText('一曲目')
  await sheet.getByRole('button', { name: '一時停止' }).click()
  await expect(sheet.getByRole('button', { name: '再生', exact: true })).toBeVisible()
  await sheet.getByRole('button', { name: '次の曲' }).click()
  await expect(sheet).toContainText('二曲目')

  await sheet.getByRole('button', { name: '再生画面を閉じる' }).click()
  await expect(sheet).toBeHidden()
  await expect(bar).toContainText('二曲目')

  await open.click()
  await expect(sheet).toBeVisible()
  await page.keyboard.press('Escape')
  await expect(sheet).toBeHidden()

  // 上に空けた後ろの画面を押しても閉じる
  await open.click()
  await expect(sheet).toBeVisible()
  await page.getByTestId('player-sheet-backdrop').click({ position: { x: 10, y: 5 } })
  await expect(sheet).toBeHidden()

  // パネルのどこでも、少し引いただけでは閉じず、大きく引けば閉じる。引き出し終わる（上端から 16px で止まる）のを待ってから触る
  await open.click()
  await expect.poll(async () => (await sheet.boundingBox())?.y).toBe(16)
  const title = sheet.getByText('二曲目')
  const box = (await title.boundingBox())!
  const x = box.x + box.width / 2
  const y = box.y + box.height / 2
  await dragDown(page, x, y, 60)
  await expect(sheet).toBeVisible()

  // シークのバーなど、引く対象から外した部品の上で始めた操作では閉じない
  await title.evaluate(el => el.setAttribute('data-sheet-no-drag', ''))
  await dragDown(page, x, y, 300)
  await expect(sheet).toBeVisible()
  await title.evaluate(el => el.removeAttribute('data-sheet-no-drag'))

  await dragDown(page, x, y, 300)
  await expect(sheet).toBeHidden()
})

test('再生のボタンとジャケットは、再生中は強く、止めているときは弱く光る', async ({ page, isMobile }) => {
  test.skip(isMobile, '右の欄は広い画面だけ')
  await page.setViewportSize({ width: 1440, height: 900 })
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3 })
  await mockStream(page)
  await page.goto('/library/albums/al-1')
  await page.getByRole('button', { name: 'すべて再生' }).click()

  const panel = page.getByTestId('player-panel')
  const pause = panel.getByRole('button', { name: '一時停止' })
  await expect(pause).toHaveClass(/glow-high/)
  await pause.click()
  await expect(panel.getByRole('button', { name: '再生', exact: true })).toHaveClass(/glow-low/)
})

test('スマホの再生画面だけ、再生中の曲のジャケットをぼかした背景を敷く', async ({ page, isMobile }) => {
  if (!isMobile) {
    await page.setViewportSize({ width: 1440, height: 900 })
  }
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3 })
  await mockStream(page)
  await page.goto('/library/albums/al-1')
  await page.getByRole('button', { name: 'すべて再生' }).click()

  const backdrop = page.getByTestId('player-backdrop').locator('img')
  if (!isMobile) {
    // 右の欄には敷かない
    await expect(page.getByTestId('player-panel')).toContainText('一曲目')
    await expect(backdrop).toHaveCount(0)
    return
  }
  await page.getByRole('button', { name: '再生画面を開く' }).click()
  await expect(backdrop).toHaveAttribute('src', /getCoverArt\?id=al-1&/)
})

test('進み具合のバーで好きな位置に移り、キーでも動かせる', async ({ page, isMobile }) => {
  test.skip(isMobile, 'つまむ操作は PC で確かめる')
  await page.setViewportSize({ width: 1440, height: 900 })
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3 })
  await mockStream(page, { seconds: 200 })
  await page.goto('/library/albums/al-1')
  await page.getByRole('button', { name: 'すべて再生' }).click()

  const panel = page.getByTestId('player-panel')
  await panel.getByRole('button', { name: '一時停止' }).click()
  const seek = panel.getByRole('slider', { name: '再生位置' })
  const box = (await seek.boundingBox())!
  // 曲の長さは 200 秒（3:20）。半分の位置で離すと 1:40 に移る
  await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2)
  await expect(panel).toContainText('1:40')
  await expect.poll(() => page.getByTestId('audio').evaluate(el => Math.round((el as HTMLAudioElement).currentTime))).toBe(100)

  await seek.focus()
  await page.keyboard.press('ArrowRight')
  await expect(panel).toContainText('1:45')
})

test('シャッフルで残りの曲を混ぜ、オフで元の並びに戻る', async ({ page, isMobile }) => {
  test.skip(isMobile, '右の欄は広い画面だけ')
  await page.setViewportSize({ width: 1440, height: 900 })
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 3 })
  const { ids } = await mockStream(page)
  await page.goto('/library/albums/al-1')
  await page.getByRole('button', { name: 'すべて再生' }).click()

  const panel = page.getByTestId('player-panel')
  const shuffle = panel.getByRole('button', { name: 'シャッフル' })
  await shuffle.click()
  await expect(shuffle).toHaveAttribute('aria-pressed', 'true')
  // 再生中の曲はそのまま鳴らし続け、次の曲は残りの二曲のどちらか
  await expect(panel).toContainText('一曲目')
  await panel.getByRole('button', { name: '次の曲' }).click()
  await expect.poll(() => ids.length).toBe(2)
  expect(['al-1-2', 'al-1-3']).toContain(ids[1])

  // オフにすると元の並びに戻り、再生中の曲の次から続ける
  await shuffle.click()
  await expect(shuffle).toHaveAttribute('aria-pressed', 'false')
  const playing = ids[1]
  const canNext = panel.getByRole('button', { name: '次の曲' })
  if (playing === 'al-1-3') {
    await expect(canNext).toBeDisabled()
  }
  else {
    await canNext.click()
    await expect.poll(() => ids.at(-1)).toBe('al-1-3')
  }
})

test('リピートはなし、キュー全体、1 曲の順に切り替わり、キュー全体なら最後の曲の後に先頭へ戻る', async ({ page, isMobile }) => {
  test.skip(isMobile, '右の欄は広い画面だけ')
  await page.setViewportSize({ width: 1440, height: 900 })
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3 })
  const { ids } = await mockStream(page, { seconds: 0.5 })
  await page.goto('/library/albums/al-1')

  const panel = page.getByTestId('player-panel')
  await page.getByRole('button', { name: 'すべて再生' }).click()
  await panel.getByRole('button', { name: 'リピート（なし）' }).click()
  await expect(panel.getByRole('button', { name: 'リピート（キュー全体）' })).toHaveAttribute('aria-pressed', 'true')
  // 二曲のキューを鳴らし終えると、先頭に戻って鳴らし続ける
  await expect.poll(() => ids.slice(0, 3)).toEqual(['al-1-1', 'al-1-2', 'al-1-1'])

  await panel.getByRole('button', { name: 'リピート（キュー全体）' }).click()
  await expect(panel.getByRole('button', { name: 'リピート（1 曲）' })).toBeVisible()
  await panel.getByRole('button', { name: 'リピート（1 曲）' }).click()
  await expect(panel.getByRole('button', { name: 'リピート（なし）' })).toHaveAttribute('aria-pressed', 'false')
})

test('PC では音量を変えられ、選んだ音量を残す', async ({ page, isMobile }) => {
  test.skip(isMobile, '音量は PC だけ')
  await page.setViewportSize({ width: 1440, height: 900 })
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3 })
  await mockStream(page)
  await page.goto('/library/albums/al-1')

  const panel = page.getByTestId('player-panel')
  const volume = panel.getByRole('slider', { name: '音量' })
  const audioVolume = () => page.getByTestId('audio').evaluate(el => (el as HTMLAudioElement).volume)
  const box = (await volume.boundingBox())!
  await page.mouse.click(box.x + box.width / 4, box.y + box.height / 2)
  await expect.poll(audioVolume).toBeCloseTo(0.25, 1)

  await panel.getByRole('button', { name: '消音' }).click()
  await expect.poll(audioVolume).toBe(0)
  await panel.getByRole('button', { name: '消音を解除' }).click()
  await expect.poll(audioVolume).toBeCloseTo(0.25, 1)

  await page.reload()
  await expect.poll(audioVolume).toBeCloseTo(0.25, 1)
})
