<script setup lang="ts">
import type { Component } from 'vue'
import type { Album } from '~/utils/subsonic'
import { Disc3, LayoutGrid, List } from 'lucide-vue-next'

useHead({ title: 'アルバム' })

const subsonic = useSubsonic()
const dev = useDev()

const { items, done, loading, error, loadMore } = useInfiniteList<Album>('albums:name', async (offset, size) => {
  const res = await subsonic<{ albumList2: { album?: Album[] } }>('getAlbumList2', {
    type: 'alphabeticalByName',
    size,
    offset,
  })
  return res.albumList2.album ?? []
})

// 表示の切り替えと並び順は、未実装の選択肢があるので開発モードのときだけ出す（docs/web.md の「開発」）
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
      <template v-if="dev" #default>
        <div class="flex flex-wrap items-center gap-2" data-testid="album-controls">
          <SegmentedControl v-model="view" label="表示" :options="views" icon-only />
          <select
            v-model="sort"
            aria-label="並び順"
            class="ml-auto h-8 rounded-md border border-border-subtle bg-surface-1 px-2 text-body-sm text-fg-muted"
          >
            <option v-for="option in sorts" :key="option.value" :value="option.value" :disabled="option.disabled">
              {{ option.label }}
            </option>
          </select>
        </div>
      </template>
    </PageHeader>

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
    <EmptyState v-else-if="done && items.length === 0" :icon="Disc3" title="アルバムがありません" />
  </div>
</template>
