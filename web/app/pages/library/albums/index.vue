<script setup lang="ts">
import type { Component } from 'vue'
import type { AlbumView } from '~/utils/albums'
import { LayoutGrid, List } from 'lucide-vue-next'
import { albumSort, albumSortOptions, albumView } from '~/utils/albums'

useHead({ title: 'アルバム' })

// お気に入りだけに絞り込む（docs/web.md の「一覧」）
const favorite = useListOption('albums-favorite', false)
const view = useStoredSort('suisei-album-view', albumView)
const sort = useStoredSort('suisei-album-sort', albumSort)

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
