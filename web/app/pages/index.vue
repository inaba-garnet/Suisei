<script setup lang="ts">
import type { Album, Playlist } from '~/utils/subsonic'

/**
 * ホーム（docs/web.md の「ホーム」）。最近再生した、よく聴く、お気に入りのアルバムと、プレイリストを棚で並べる。
 * 棚ごとに読み込むので、一つが失敗してもほかの棚は出す。
 */
useHead({ title: 'ホーム' })

const SIZE = 12
const subsonic = useSubsonic()

async function albums(type: string): Promise<Album[]> {
  return (await subsonic<{ albumList2: { album?: Album[] } }>('getAlbumList2', { type, size: SIZE })).albumList2.album ?? []
}

const { data, status, refresh } = useAsyncData('home', async () => {
  const [recent, frequent, starred, playlists] = await Promise.allSettled([
    albums('recent'),
    albums('frequent'),
    // お気に入りの棚もほかの棚と同じく、最近の操作が先に来るようにお気に入りにした新しい順にする
    albums('starred'),
    subsonic<{ playlists: { playlist?: Playlist[] } }>('getPlaylists').then(res => (res.playlists.playlist ?? []).slice(0, SIZE)),
  ])
  const value = <T,>(result: PromiseSettledResult<T>) => result.status === 'fulfilled' ? result.value : undefined
  return { recent: value(recent), frequent: value(frequent), starred: value(starred), playlists: value(playlists) }
})

const shelves = computed(() => [
  { key: 'recent', title: '最近再生したアルバム', albums: data.value?.recent, more: '/library/albums?sort=recent', empty: '曲を再生すると、ここに並びます。' },
  { key: 'frequent', title: 'よく聴くアルバム', albums: data.value?.frequent, more: '/library/albums?sort=frequent', empty: '曲を再生すると、よく聴く順に並びます。' },
  { key: 'starred', title: 'お気に入りのアルバム', albums: data.value?.starred, more: '/library/albums?favorite=1', empty: 'アルバムをハートでお気に入りにすると、ここに並びます。' },
])
const failed = computed(() => !!data.value && Object.values(data.value).some(v => v === undefined))
</script>

<template>
  <div>
    <PageHeader title="ホーム" />

    <p v-if="status === 'pending' && !data" class="py-6 text-center text-body-sm text-fg-subtle">
      読み込み中…
    </p>
    <div v-else class="space-y-8" data-testid="home">
      <AlbumShelf v-for="shelf in shelves" :key="shelf.key" :title="shelf.title" :albums="shelf.albums ?? []" caption="artist" :more="shelf.more">
        <template #fallback>
          <p v-if="shelf.albums" class="text-body-sm text-fg-subtle">
            {{ shelf.empty }}
          </p>
          <p v-else class="text-body-sm text-danger">
            読み込めませんでした
          </p>
        </template>
      </AlbumShelf>

      <PlaylistShelf title="プレイリスト" :playlists="data?.playlists ?? []" more="/playlists">
        <template #fallback>
          <p v-if="data?.playlists" class="text-body-sm text-fg-subtle">
            プレイリストを作ると、ここに並びます。
          </p>
          <p v-else class="text-body-sm text-danger">
            読み込めませんでした
          </p>
        </template>
      </PlaylistShelf>

      <div v-if="failed" class="flex justify-center">
        <Button variant="secondary" @click="refresh()">
          もう一度読み込む
        </Button>
      </div>
    </div>
  </div>
</template>
