<script setup lang="ts">
import type { Album } from '~/utils/subsonic'
import { Disc3 } from 'lucide-vue-next'

/**
 * アルバムを 100 枚ずつ読み足すグリッド（docs/web.md の「一覧」）。`favorite` ならお気に入りだけを、お気に入りにした新しい順に並べる。
 * 読み込んだ一覧と位置は絞り込みごとに残す。
 */
const props = defineProps<{ favorite: boolean }>()

const subsonic = useSubsonic()

const { items, done, loading, error, loadMore } = useInfiniteList<Album>(props.favorite ? 'albums:starred' : 'albums:name', async (offset, size) => {
  const res = await subsonic<{ albumList2: { album?: Album[] } }>('getAlbumList2', {
    type: props.favorite ? 'starred' : 'alphabeticalByName',
    size,
    offset,
  })
  return res.albumList2.album ?? []
})
</script>

<template>
  <div>
    <AlbumGrid :albums="items" :more="!done && !loading && !error" @more="loadMore" />

    <p v-if="loading" class="py-6 text-center text-body-sm text-fg-subtle">
      読み込み中…
    </p>
    <div v-else-if="error" class="flex flex-col items-center gap-3 py-6">
      <p class="text-body text-danger">
        アルバムを読み込めませんでした
      </p>
      <Button variant="secondary" @click="loadMore">
        もう一度読み込む
      </Button>
    </div>
    <EmptyState v-else-if="done && items.length === 0" :icon="Disc3" :title="favorite ? 'お気に入りのアルバムがありません' : 'アルバムがありません'" />
  </div>
</template>
