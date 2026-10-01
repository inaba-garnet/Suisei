<script setup lang="ts">
import type { FavoriteSort } from '~/utils/favorites'
import type { Song } from '~/utils/subsonic'
import { Heart } from 'lucide-vue-next'
import { favoriteSongSort } from '~/utils/favorites'

/** お気に入りの曲を `sort` の順に並べる（docs/web.md の「一覧」）。読み込んだ一覧と位置は並び順ごとに残す。 */
const props = defineProps<{ sort: FavoriteSort }>()

const subsonic = useSubsonic()

// getStarred2 は全件を一度に返すので、一回で読み終える
const { items, done, loading, error, loadMore } = useInfiniteList<Song>(`favorites:${props.sort}`, async () => {
  const songSort = favoriteSongSort(props.sort)
  const res = await subsonic<{ starred2: { song?: Song[] } }>('getStarred2', songSort ? { songSort } : {})
  const songs = res.starred2.song ?? []
  return props.sort === 'oldest' ? [...songs].reverse() : songs
}, Number.MAX_SAFE_INTEGER)
</script>

<template>
  <div>
    <SongList :songs="items" :more="false" />

    <p v-if="loading" class="py-6 text-center text-body-sm text-fg-subtle">
      読み込み中…
    </p>
    <div v-else-if="error" class="flex flex-col items-center gap-3 py-6">
      <p class="text-body text-danger">
        お気に入りを読み込めませんでした
      </p>
      <Button variant="secondary" @click="loadMore">
        もう一度読み込む
      </Button>
    </div>
    <EmptyState v-else-if="done && items.length === 0" :icon="Heart" title="お気に入りの曲がありません" />
  </div>
</template>
