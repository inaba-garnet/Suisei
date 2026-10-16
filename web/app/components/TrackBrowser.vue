<script setup lang="ts">
import type { TrackSort } from '~/utils/tracks'
import type { Song } from '~/utils/subsonic'
import { useElementSize } from '@vueuse/core'
import { Music } from 'lucide-vue-next'

/**
 * 曲を `sort` の順に 100 曲ずつ読む一覧（docs/web.md の「一覧」）。`genre` があればそのジャンルの曲、なければ全曲。
 * 全曲は右端に頭文字の索引を置き、見えた範囲のページを読む。ジャンルの曲は末尾に近づいたら読み足す。
 * 読み込んだ一覧と位置は並び順ごとに残す。
 */
const props = defineProps<{ sort: TrackSort, genre?: string }>()

const subsonic = useSubsonic()
const headingIndex = useHeadingIndex()

const key = props.genre === undefined ? `tracks:${props.sort}` : `genre:${props.genre}:${props.sort}`
async function fetchPage(offset: number, size: number) {
  if (props.genre !== undefined) {
    const res = await subsonic<{ songsByGenre: { song?: Song[] } }>('getSongsByGenre', {
      genre: props.genre,
      count: size,
      offset,
      songSort: props.sort,
    })
    return res.songsByGenre.song ?? []
  }
  const res = await subsonic<{ searchResult3: { song?: Song[] } }>('search3', {
    query: '',
    artistCount: 0,
    albumCount: 0,
    songCount: size,
    songOffset: offset,
    songSort: props.sort,
  })
  return res.searchResult3.song ?? []
}

// 全曲だけ、頭文字の索引を置く。サーバーの索引はジャンルの絞り込みを受けないため
const loadIndex = props.genre === undefined
  ? () => headingIndex({ list: 'tracks', sort: props.sort })
  : undefined
const { items, headings, indexed, loading, error, done, more, loadMore, ensure, retry } = useBrowseList<Song>(key, fetchPage, loadIndex)

const listView = ref<{ scrollToItem: (index: number) => void }>()
const wrapper = ref<HTMLElement>()
const { height } = useElementSize(wrapper)

function jump(heading: string) {
  const offset = headings.value.find(h => h.heading === heading)?.offset
  if (offset !== undefined) {
    listView.value?.scrollToItem(offset)
  }
}
</script>

<template>
  <div>
    <div ref="wrapper">
      <SongList ref="listView" :songs="items" :more="more" @more="loadMore" @range="ensure" />
    </div>
    <IndexRail v-if="indexed" :headings="headings.map(h => h.heading)" :content-height="height" @select="jump" />

    <p v-if="loading" class="py-6 text-center text-body-sm text-fg-subtle">
      読み込み中…
    </p>
    <div v-else-if="error" class="flex flex-col items-center gap-3 py-6">
      <p class="text-body text-danger">
        曲を読み込めませんでした
      </p>
      <Button variant="secondary" @click="retry">
        もう一度読み込む
      </Button>
    </div>
    <EmptyState v-else-if="done && items.length === 0" :icon="Music" title="曲がありません" />
  </div>
</template>
