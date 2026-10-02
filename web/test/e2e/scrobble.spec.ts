import type { Page } from '@playwright/test'
import { expect, test } from '@playwright/test'
import { mockApi, mockLibrary, mockScrobble, mockStream, recordMediaSession } from './api'

/** OS のロック画面の操作の代わりに、Media Session に渡したハンドラを呼ぶ。 */
function mediaAction(page: Page, action: string, details: object = {}) {
  return page.evaluate(([action, details]) => {
    const handlers = (window as unknown as { mediaSessionHandlers: Record<string, ((d: object) => void) | null> }).mediaSessionHandlers
    handlers[action as string]!({ action, ...details as object })
  }, [action, details] as const)
}

function mediaHandler(page: Page, action: string) {
  return page.evaluate(action => !!(window as unknown as { mediaSessionHandlers: Record<string, unknown> }).mediaSessionHandlers[action], action)
}

test('曲の半分まで聴いたら、一曲につき一度だけ再生回数に数える', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3 })
  await mockStream(page, { seconds: 2 })
  const { calls } = await mockScrobble(page)
  await page.goto('/library/albums/al-1')

  const before = Date.now()
  await page.getByRole('button', { name: 'すべて再生' }).click()
  await expect.poll(() => calls.map(c => c.get('id')), { timeout: 10_000 }).toEqual(['al-1-1', 'al-1-2'])
  for (const call of calls) {
    expect(call.get('submission')).toBe('true')
    // 聴き始めた時刻（ミリ秒）を渡す
    expect(Number(call.get('time'))).toBeGreaterThanOrEqual(before)
  }
  // 再生を始めたときの submission=false は送らない
  await page.waitForTimeout(500)
  expect(calls).toHaveLength(2)
})

test('シークで飛ばした分は聴いたうちに入れない', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3 })
  await mockStream(page, { seconds: 30 })
  const { calls } = await mockScrobble(page)
  await page.goto('/library/albums/al-1')

  await page.getByRole('button', { name: 'すべて再生' }).click()
  const bar = page.getByTestId('player-bar')
  await expect(bar).toContainText('一曲目')
  await expect.poll(() => page.getByTestId('audio').evaluate((el: HTMLAudioElement) => el.currentTime)).toBeGreaterThan(1)
  await page.getByTestId('audio').evaluate((el: HTMLAudioElement) => {
    el.currentTime = 28
  })
  // 最後まで鳴らしても、聴いたのは 3 秒ほどなので数えない
  await expect(bar).toContainText('二曲目', { timeout: 10_000 })
  expect(calls).toHaveLength(0)
})

test('Media Session に再生中の曲を出し、ロック画面の操作を受ける', async ({ page }) => {
  await recordMediaSession(page)
  await mockApi(page, { loggedIn: true })
  await mockLibrary(page, { count: 3, songs: 3 })
  await mockStream(page)
  await mockScrobble(page)
  await page.goto('/library/albums/al-1')

  const session = () => page.evaluate(() => {
    const { metadata, playbackState } = navigator.mediaSession
    return { title: metadata?.title, artist: metadata?.artist, album: metadata?.album, artwork: metadata?.artwork.map(a => a.sizes), playbackState }
  })
  expect(await session()).toEqual({ title: undefined, artist: undefined, album: undefined, artwork: undefined, playbackState: 'none' })

  await page.getByRole('button', { name: 'すべて再生' }).click()
  const bar = page.getByTestId('player-bar')
  await expect(bar).toContainText('一曲目')
  await expect.poll(session).toEqual({ title: '一曲目', artist: 'アーティスト 1', album: 'アルバム 001', artwork: ['256x256', '512x512'], playbackState: 'playing' })
  // 10 秒の戻しと送りは受けない
  expect(await mediaHandler(page, 'seekbackward')).toBe(false)
  expect(await mediaHandler(page, 'seekforward')).toBe(false)

  await mediaAction(page, 'pause')
  await expect.poll(async () => (await session()).playbackState).toBe('paused')
  await mediaAction(page, 'play')
  await expect.poll(async () => (await session()).playbackState).toBe('playing')

  await mediaAction(page, 'seekto', { seekTime: 10 })
  await expect.poll(() => page.getByTestId('audio').evaluate((el: HTMLAudioElement) => el.currentTime)).toBeGreaterThanOrEqual(10)

  await mediaAction(page, 'nexttrack')
  await expect(bar).toContainText('二曲目')
  await mediaAction(page, 'nexttrack')
  await expect(bar).toContainText('曲 3')
  // キューの最後では次の曲を受けない
  await expect.poll(() => mediaHandler(page, 'nexttrack')).toBe(false)
  await mediaAction(page, 'previoustrack')
  await expect(bar).toContainText('二曲目')
})
