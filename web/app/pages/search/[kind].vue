<script setup lang="ts">
import type { Album, Artist, Song } from '~/utils/subsonic'
import type { SearchResult } from '~/utils/search'
import { SearchX } from 'lucide-vue-next'
import { searchKind, searchKinds, searchQuery } from '~/utils/search'

/** 検索の結果の一つの種類を、100 件ずつ読み足して並べる（docs/web.md の「検索」）。 */
const route = useRoute()
const kind = searchKind(route.params.kind)
if (!kind) {
  throw createError({ statusCode: 404, statusMessage: 'Not Found', fatal: true })
}
const label = searchKinds.find(k => k.kind === kind)!.label
const query = searchQuery(route.query.q)
// 空の検索語は全件を返すので、検索語がなければ検索の画面に移す
if (!query) {
  await navigateTo('/search', { replace: true })
}

useHead({ title: `${label}の検索結果` })

const subsonic = useSubsonic()
const counts = { artists: 'artistCount', albums: 'albumCount', songs: 'songCount' } as const
const offsets = { artists: 'artistOffset', albums: 'albumOffset', songs: 'songOffset' } as const

const { items, done, loading, error, loadMore } = useInfiniteList<Artist | Album | Song>(`search:${kind}:${query}`, async (offset, size) => {
  const res = await subsonic<{ searchResult3: SearchResult }>('search3', {
    query,
    artistCount: 0,
    albumCount: 0,
    songCount: 0,
    [counts[kind]]: size,
    [offsets[kind]]: offset,
  })
  const r = res.searchResult3
  return (kind === 'artists' ? r.artist : kind === 'albums' ? r.album : r.song) ?? []
})
const more = computed(() => !done.value && !loading.value && !error.value)
</script>

<template>
  <div>
    <PageHeader :title="label" :description="`「${query}」`" :back="{ to: `/search?q=${encodeURIComponent(query)}` }" />

    <ArtistRows v-if="kind === 'artists'" :artists="items as Artist[]" :more="more" @more="loadMore" />
    <AlbumGrid v-else-if="kind === 'albums'" :albums="items as Album[]" :more="more" @more="loadMore" />
    <SongList v-else :songs="items as Song[]" :more="more" @more="loadMore" />

    <p v-if="loading" class="py-6 text-center text-body-sm text-fg-subtle">
      読み込み中…
    </p>
    <div v-else-if="error" class="flex flex-col items-center gap-3 py-6">
      <p class="text-body text-danger">
        検索できませんでした
      </p>
      <Button variant="secondary" @click="loadMore">
        もう一度読み込む
      </Button>
    </div>
    <EmptyState v-else-if="done && items.length === 0" :icon="SearchX" :title="`「${query}」は見つかりませんでした`" />
  </div>
</template>
