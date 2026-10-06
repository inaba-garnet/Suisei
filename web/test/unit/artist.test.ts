import { describe, expect, it } from 'vitest'
import { albumsAsAlbumArtist, artistSort, artistView, groupByAlbum, sortAlbums, sortGroups, sortSongs, viewForRoles } from '~/utils/artist'

describe('artistView', () => {
  it('知らない値はアルバムアーティストの表示にする', () => {
    expect(artistView('tracks')).toBe('tracks')
    expect(artistView('composer')).toBe('composer')
    expect(artistView(undefined)).toBe('albums')
    expect(artistView('x')).toBe('albums')
  })
})

describe('viewForRoles', () => {
  it('曲のアーティスト、アルバムアーティスト、作曲家の順に表示を選ぶ', () => {
    expect(viewForRoles({ roles: ['albumartist', 'artist', 'composer'] })).toBe('tracks')
    expect(viewForRoles({ roles: ['albumartist', 'composer'] })).toBe('albums')
    expect(viewForRoles({ roles: ['composer', 'arranger'] })).toBe('composer')
    expect(viewForRoles({ roles: ['lyricist'] })).toBe('composer')
  })

  it('役割がなければ曲のアーティストの表示にする', () => {
    expect(viewForRoles({})).toBe('tracks')
  })
})

describe('albumsAsAlbumArtist', () => {
  it('アルバムアーティストになっているアルバムだけを残す', () => {
    const album = (id: string, artists: string[]) => ({ id, name: id, songCount: 1, duration: 1, artists: artists.map(a => ({ id: a, name: a })) })
    const albums = [album('a', ['ar-1']), album('b', ['ar-2']), album('c', ['ar-2', 'ar-1'])]
    expect(albumsAsAlbumArtist(albums, 'ar-1').map(a => a.id)).toEqual(['a', 'c'])
  })
})

describe('groupByAlbum', () => {
  it('続いて並ぶ同じアルバムの曲を一つの見出しにまとめる', () => {
    const song = (id: string, albumId: string) => ({ id, title: id, albumId, album: `${albumId} の名前`, duration: 1 })
    const groups = groupByAlbum([song('1', 'a'), song('2', 'a'), song('3', 'b')])
    expect(groups.map(g => [g.album, g.songs.map(s => s.id)])).toEqual([
      ['a の名前', ['1', '2']],
      ['b の名前', ['3']],
    ])
  })
})

describe('並べ替え', () => {
  const album = (id: string, year?: number, sortName?: string) => ({ id, name: id, sortName, year, songCount: 1, duration: 1 })
  const albums = [album('b', 2010), album('a', 2020), album('c'), album('d', 2020, 'エー')]

  it('知らない並び順は新しい順にする', () => {
    expect(artistSort('oldest')).toBe('oldest')
    expect(artistSort('x')).toBe('newest')
  })

  it('アルバムは年で並べ、同じ年は読みの順、年のないものはどちらの向きでも最後にする', () => {
    expect(sortAlbums(albums, 'newest').map(a => a.id)).toEqual(['a', 'd', 'b', 'c'])
    expect(sortAlbums(albums, 'oldest').map(a => a.id)).toEqual(['b', 'a', 'd', 'c'])
    expect(sortAlbums(albums, 'name').map(a => a.id)).toEqual(['a', 'b', 'c', 'd'])
  })

  it('曲の見出しはアルバムの年で並べ、見出しの中の順は変えない', () => {
    const song = (id: string, albumId: string) => ({ id, title: id, albumId, album: albumId, duration: 1 })
    const groups = groupByAlbum([song('1', 'b'), song('2', 'b'), song('3', 'a')])
    const sorted = sortGroups(groups, 'newest', albums)
    expect(sorted.map(g => [g.albumId, g.songs.map(s => s.id)])).toEqual([['a', ['3']], ['b', ['1', '2']]])
  })

  it('曲は年で並べ、同じ年は元の順のまま、曲名順では読みで並べる', () => {
    const song = (id: string, year?: number, sortName?: string) => ({ id, title: id, sortName, year, duration: 1 })
    const songs = [song('y', 2010), song('x', 2010), song('z', 2020), song('w', undefined, 'ア')]
    expect(sortSongs(songs, 'newest').map(s => s.id)).toEqual(['z', 'y', 'x', 'w'])
    expect(sortSongs(songs, 'oldest').map(s => s.id)).toEqual(['y', 'x', 'z', 'w'])
    expect(sortSongs(songs, 'name').map(s => s.id)).toEqual(['x', 'y', 'z', 'w'])
  })
})
