import { expect, test } from '@playwright/test'
import { mockApi, mockSearch, mockStream } from './api'

test('打ち終えてから一度だけ検索し、検索語を URL に入れて区切りごとに出す', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  const { requests } = await mockSearch(page)
  await page.goto('/search')
  await expect(page.getByText('ライブラリを探す')).toBeVisible()

  await page.getByRole('searchbox', { name: '検索語' }).pressSequentially('ひかり', { delay: 50 })
  await expect(page).toHaveURL('/search?q=%E3%81%B2%E3%81%8B%E3%82%8A')
  const results = page.getByTestId('search-results')
  await expect(results.getByRole('region', { name: 'アーティスト' }).getByRole('listitem')).toHaveCount(5)
  await expect(results.getByRole('region', { name: 'アルバム' }).getByRole('listitem')).toHaveCount(10)
  // 曲の行は見える分だけ描くので、末尾まで送って 10 曲目まであり 11 曲目がないことを見る
  const songs = results.getByRole('region', { name: '曲' })
  await songs.getByText('ひかりの曲 000').waitFor()
  await page.getByTestId('scroller').evaluate((el) => {
    el.scrollTop = 1e6
  })
  await expect(songs.getByText('ひかりの曲 009')).toBeVisible()
  await expect(songs.getByText('ひかりの曲 010')).toHaveCount(0)
  // 打っている間は呼ばない。出す件数より 1 件多く頼む
  expect(requests).toHaveLength(1)
  expect(requests[0]?.get('query')).toBe('ひかり')
  expect(requests[0]?.get('artistCount')).toBe('6')
  expect(requests[0]?.get('albumCount')).toBe('11')
  expect(requests[0]?.get('songCount')).toBe('11')
  await expect(results.getByRole('link', { name: 'すべて見る' })).toHaveCount(3)

  // アーティストは役割に合う表示で開く
  const views = ['tracks', 'albums', 'composer', 'tracks']
  for (const [i, view] of views.entries()) {
    await expect(results.getByRole('link', { name: `ひかりのアーティスト ${i}` })).toHaveAttribute('href', `/library/artists/ar-${i}?view=${view}`)
  }
  await results.getByRole('link', { name: 'ひかりのアーティスト 2' }).click()
  await expect(page).toHaveURL('/library/artists/ar-2?view=composer')
})

test('件数に収まる種類には「すべて見る」を出さず、見つからなければそう出す', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  await mockSearch(page, { artists: 2, albums: 3, songs: 30 })
  await page.goto('/search?q=ひかり')
  const results = page.getByTestId('search-results')
  await expect(results.getByRole('region', { name: 'アーティスト' }).getByRole('listitem')).toHaveCount(2)
  const more = results.getByRole('link', { name: 'すべて見る' })
  await expect(more).toHaveCount(1)
  await expect(more).toHaveAttribute('href', '/search/songs?q=%E3%81%B2%E3%81%8B%E3%82%8A')

  const box = page.getByRole('searchbox', { name: '検索語' })
  await expect(box).toHaveValue('ひかり')
  await box.fill('ない')
  await expect(page.getByText('「ない」は見つかりませんでした')).toBeVisible()
  await box.fill('')
  await expect(page).toHaveURL('/search')
  await expect(page.getByText('ライブラリを探す')).toBeVisible()
})

test('「すべて見る」は 100 件ずつ読み足し、「戻る」で検索の結果に戻る', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  const { requests } = await mockSearch(page)
  await page.goto('/search?q=ひかり')
  await page.getByRole('region', { name: '曲' }).getByRole('link', { name: 'すべて見る' }).click()

  await expect(page).toHaveURL('/search/songs?q=%E3%81%B2%E3%81%8B%E3%82%8A')
  await expect(page.getByRole('heading', { name: '曲' })).toBeVisible()
  await expect(page.getByTestId('songs').getByText('ひかりの曲 000')).toBeVisible()
  const first = requests.at(-1)!
  expect(first.get('songCount')).toBe('100')
  expect(first.get('songOffset')).toBe('0')
  expect(first.get('artistCount')).toBe('0')
  expect(first.get('albumCount')).toBe('0')
  await page.getByTestId('scroller').evaluate((el) => {
    el.scrollTop = 1e6
  })
  await expect.poll(() => requests.at(-1)?.get('songOffset')).toBe('100')

  await page.getByRole('button', { name: '戻る', exact: true }).click()
  await expect(page).toHaveURL('/search?q=%E3%81%B2%E3%81%8B%E3%82%8A')
  await expect(page.getByRole('searchbox', { name: '検索語' })).toHaveValue('ひかり')
  await expect(page.getByTestId('search-results')).toBeVisible()
})

test('検索語のない「すべて見る」は検索の画面に移す', async ({ page }) => {
  await mockApi(page, { loggedIn: true })
  const { requests } = await mockSearch(page)
  await page.goto('/search/songs')
  await expect(page).toHaveURL('/search')
  expect(requests).toHaveLength(0)
})

test('結果の曲を押すと、その曲から結果の曲を順に鳴らす', async ({ page, isMobile }) => {
  test.skip(isMobile, '次の曲のボタンは PC だけ')
  await mockApi(page, { loggedIn: true })
  await mockSearch(page)
  const { ids } = await mockStream(page)
  await page.goto('/search?q=ひかり')
  await page.getByTestId('songs').getByText('ひかりの曲 002').click()
  const bar = page.getByTestId('player-bar')
  await expect(bar).toContainText('ひかりの曲 002')
  await bar.getByRole('button', { name: '次の曲' }).click()
  await expect(bar).toContainText('ひかりの曲 003')
  await expect.poll(() => ids).toEqual(['tr-2', 'tr-3'])
})

test('検索のタブを押し直すと検索語を消す', async ({ page, isMobile }) => {
  test.skip(!isMobile, '下のタブはスマホだけ')
  await mockApi(page, { loggedIn: true })
  await mockSearch(page)
  await page.goto('/search?q=ひかり')
  await expect(page.getByTestId('search-results')).toBeVisible()
  await page.getByRole('navigation', { name: 'メイン' }).getByRole('link', { name: '検索' }).click()
  await expect(page).toHaveURL('/search')
  await expect(page.getByRole('searchbox', { name: '検索語' })).toHaveValue('')
  await expect(page.getByText('ライブラリを探す')).toBeVisible()
})

test('入力欄の文字はスマホで 16px にし、iPhone で触れても拡大させない', async ({ page, isMobile }) => {
  await mockApi(page, { loggedIn: true })
  await mockSearch(page)
  await page.goto('/search')
  await expect(page.getByRole('searchbox', { name: '検索語' })).toHaveCSS('font-size', isMobile ? '16px' : '14px')
})
