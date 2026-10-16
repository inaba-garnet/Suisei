<script setup lang="ts">
import type { AlbumSort, AlbumView } from '~/utils/albums'
import type { Album } from '~/utils/subsonic'
import { useElementSize } from '@vueuse/core'
import { Disc3 } from 'lucide-vue-next'
import { albumListParams } from '~/utils/albums'

/**
 * アルバムを `sort` の順に 100 枚ずつ読む一覧（docs/web.md の「一覧」）。`favorite` ならお気に入りだけに絞る。
 * 名前順とアーティスト順は右端に頭文字の索引を置き、見えた範囲のページを読む。ほかの順は末尾に近づいたら読み足す。
 * 読み込んだ一覧と位置は、並び順と絞り込みごとに残す。
 */
const props = defineProps<{ sort: AlbumSort, favorite: boolean, view: AlbumView }>()

const subsonic = useSubsonic()
const headingIndex = useHeadingIndex()

const key = `albums:${props.sort}:${props.favorite ? 'starred' : 'all'}`
async function fetchPage(offset: number, size: number) {
  const res = await subsonic<{ albumList2: { album?: Album[] } }>('getAlbumList2', {
    ...albumListParams(props.sort),
    // サーバーの独自の引数。並べ方はそのままで、お気に入りだけに絞る
    ...(props.favorite && { starred: 'true' }),
    size,
    offset,
  })
  return res.albumList2.album ?? []
}

// 名前順とアーティスト順だけ、頭文字の索引を置く。ほかの順では頭文字で探さないため
const loadIndex = props.sort === 'name' || props.sort === 'artist'
  ? () => headingIndex({ list: 'albums', sort: props.sort, ...(props.favorite && { starred: 'true' }) })
  : undefined
const { items, headings, indexed, loading, error, done, more, loadMore, ensure, retry } = useBrowseList<Album>(key, fetchPage, loadIndex)

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
      <AlbumGrid v-if="view === 'grid'" ref="listView" :albums="items" :more="more" @more="loadMore" @range="ensure" />
      <AlbumList v-else ref="listView" :albums="items" :more="more" @more="loadMore" @range="ensure" />
    </div>
    <IndexRail v-if="indexed" :headings="headings.map(h => h.heading)" :content-height="height" @select="jump" />

    <p v-if="loading" class="py-6 text-center text-body-sm text-fg-subtle">
      読み込み中…
    </p>
    <div v-else-if="error" class="flex flex-col items-center gap-3 py-6">
      <p class="text-body text-danger">
        アルバムを読み込めませんでした
      </p>
      <Button variant="secondary" @click="retry">
        もう一度読み込む
      </Button>
    </div>
    <EmptyState v-else-if="done && items.length === 0" :icon="Disc3" :title="favorite ? 'お気に入りのアルバムがありません' : 'アルバムがありません'" />
  </div>
</template>
