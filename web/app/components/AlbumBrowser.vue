<script setup lang="ts">
import type { AlbumSort, AlbumView } from '~/utils/albums'
import type { Album } from '~/utils/subsonic'
import { Disc3 } from 'lucide-vue-next'
import { albumListParams } from '~/utils/albums'

/**
 * アルバムを `sort` の順に 100 枚ずつ読み足す一覧（docs/web.md の「一覧」）。`favorite` ならお気に入りだけに絞る。
 * 読み込んだ一覧と位置は、並び順と絞り込みごとに残す。
 */
const props = defineProps<{ sort: AlbumSort, favorite: boolean, view: AlbumView }>()

const subsonic = useSubsonic()

const key = `albums:${props.sort}:${props.favorite ? 'starred' : 'all'}`
const { items, done, loading, error, loadMore } = useInfiniteList<Album>(key, async (offset, size) => {
  const res = await subsonic<{ albumList2: { album?: Album[] } }>('getAlbumList2', {
    ...albumListParams(props.sort),
    // サーバーの独自の引数。並べ方はそのままで、お気に入りだけに絞る
    ...(props.favorite && { starred: 'true' }),
    size,
    offset,
  })
  return res.albumList2.album ?? []
})
</script>

<template>
  <div>
    <AlbumGrid v-if="view === 'grid'" :albums="items" :more="!done && !loading && !error" @more="loadMore" />
    <AlbumList v-else :albums="items" :more="!done && !loading && !error" @more="loadMore" />

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
