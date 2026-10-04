<script setup lang="ts">
import type { Component } from 'vue'
import type { AlbumView } from '~/utils/albums'
import { LayoutGrid, List } from 'lucide-vue-next'
import { albumSort, albumSortOptions, albumView, parseAlbumSort } from '~/utils/albums'

useHead({ title: 'アルバム' })

// ホームの「すべて見る」は、並び順（`?sort=`）とお気に入りの絞り込み（`?favorite=1`）を URL で渡す（docs/web.md の「ホーム」）
const route = useRoute()

// お気に入りだけに絞り込む（docs/web.md の「一覧」）
const favorite = useListOption('albums-favorite', route.query.favorite === '1')
const view = useStoredSort('suisei-album-view', albumView)
const stored = useStoredSort('suisei-album-sort', albumSort)
// URL の並び順はこの画面の間だけ使い、残さない。次にライブラリから開いたときは、残した並び順に戻すため。選び直したら残す
const override = ref(parseAlbumSort(route.query.sort))
const sort = computed({
  get: () => override.value ?? stored.value,
  set: (value) => {
    override.value = undefined
    stored.value = value
  },
})

const views: { value: AlbumView, label: string, icon: Component }[] = [
  { value: 'grid', label: 'グリッド', icon: LayoutGrid },
  { value: 'list', label: 'リスト', icon: List },
]
</script>

<template>
  <div>
    <PageHeader title="アルバム" :back="{ to: '/library', label: 'ライブラリ', mobileOnly: true }">
      <div class="flex flex-wrap items-center gap-2" data-testid="album-controls">
        <FavoriteToggle v-model="favorite" />
        <SegmentedControl v-model="view" label="表示" :options="views" icon-only />
        <SortSelect v-model="sort" :options="albumSortOptions" class="ml-auto" />
      </div>
    </PageHeader>

    <AlbumBrowser :key="`${sort}:${favorite}`" :sort="sort" :favorite="favorite" :view="view" />
  </div>
</template>
