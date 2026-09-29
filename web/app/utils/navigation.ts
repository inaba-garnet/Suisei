import type { Component } from 'vue'
import { Disc3, Heart, House, Library, ListMusic, MicVocal, Music, Search, Settings, Shapes } from 'lucide-vue-next'

export interface NavItem {
  to: string
  label: string
  icon: Component
}

/** スマホの下のタブと、PC のサイドバーの項目（docs/web.md）。 */
export const mainNav: NavItem[] = [
  { to: '/', label: 'ホーム', icon: House },
  { to: '/search', label: '検索', icon: Search },
  { to: '/library', label: 'ライブラリ', icon: Library },
  { to: '/playlists', label: 'プレイリスト', icon: ListMusic },
  { to: '/settings', label: '設定', icon: Settings },
]

/** ライブラリの中の一覧。 */
export const libraryNav: NavItem[] = [
  { to: '/library/albums', label: 'アルバム', icon: Disc3 },
  { to: '/library/artists', label: 'アーティスト', icon: MicVocal },
  { to: '/library/tracks', label: 'トラック', icon: Music },
  { to: '/library/genres', label: 'ジャンル', icon: Shapes },
  { to: '/library/favorites', label: 'お気に入り', icon: Heart },
]

/** `path` が `to` の画面か、その下の画面か。`/` はホームだけを指す。 */
export function isUnder(path: string, to: string): boolean {
  if (to === '/') {
    return path === '/'
  }
  return path === to || path.startsWith(`${to}/`)
}
