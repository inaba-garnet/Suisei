<script setup lang="ts">
import type { Component } from 'vue'
import { LayoutGrid, List } from 'lucide-vue-next'

useHead({ title: 'アルバム' })

const dev = useDev()

// お気に入りだけに絞り込む（docs/web.md の「一覧」）
const favorite = useListOption('albums-favorite', false)

type View = 'grid' | 'list'
const view = ref<View>('grid')
const views: { value: View, label: string, icon: Component, disabled?: boolean }[] = [
  { value: 'grid', label: 'グリッド', icon: LayoutGrid },
  { value: 'list', label: 'リスト', icon: List, disabled: true },
]
const sorts = [
  { value: 'name', label: '名前順' },
  { value: 'newest', label: '新着順', disabled: true },
  { value: 'artist', label: 'アーティスト順', disabled: true },
  { value: 'year', label: '年順', disabled: true },
]
const sort = ref('name')
</script>

<template>
  <div>
    <PageHeader title="アルバム" :back="{ to: '/library', label: 'ライブラリ', mobileOnly: true }">
      <div class="flex flex-wrap items-center gap-2" data-testid="album-controls">
        <FavoriteToggle v-model="favorite" />
        <!-- 表示の切り替えと並び順は、未実装の選択肢があるので開発モードのときだけ出す（docs/web.md の「開発」） -->
        <template v-if="dev">
          <SegmentedControl v-model="view" label="表示" :options="views" icon-only />
          <SortSelect v-model="sort" :options="sorts" class="ml-auto" />
        </template>
      </div>
    </PageHeader>

    <AlbumBrowser :key="String(favorite)" :favorite="favorite" />
  </div>
</template>
