<script setup lang="ts">
import type { TrackSort } from '~/utils/tracks'
import type { Song } from '~/utils/subsonic'
import { Music } from 'lucide-vue-next'

/** 全曲を `sort` の順に読み足す一覧（docs/web.md の「一覧」）。読み込んだ一覧と位置は並び順ごとに残す。 */
const props = defineProps<{ sort: TrackSort }>()

const subsonic = useSubsonic()

const { items, done, loading, error, loadMore } = useInfiniteList<Song>(`tracks:${props.sort}`, async (offset, size) => {
  const res = await subsonic<{ searchResult3: { song?: Song[] } }>('search3', {
    query: '',
    artistCount: 0,
    albumCount: 0,
    songCount: size,
    songOffset: offset,
    songSort: props.sort,
  })
  return res.searchResult3.song ?? []
})
</script>

<template>
  <div>
    <SongList :songs="items" :more="!done && !loading && !error" @more="loadMore" />

    <p v-if="loading" class="py-6 text-center text-body-sm text-fg-subtle">
      読み込み中…
    </p>
    <div v-else-if="error" class="flex flex-col items-center gap-3 py-6">
      <p class="text-body text-danger">
        曲を読み込めませんでした
      </p>
      <Button variant="secondary" @click="loadMore">
        もう一度読み込む
      </Button>
    </div>
    <EmptyState v-else-if="done && items.length === 0" :icon="Music" title="曲がありません" />
  </div>
</template>
