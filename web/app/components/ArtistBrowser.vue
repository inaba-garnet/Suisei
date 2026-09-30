<script setup lang="ts">
import type { Component } from 'vue'
import type { ArtistView } from '~/utils/artist'
import type { ArtistIndex } from '~/utils/subsonic'

/**
 * `role` の役割を持つアーティストの一覧（docs/web.md の「一覧」）。`role` はサーバーの `getArtists` の独自の引数で、カンマ区切りで複数渡せる。
 * 読み込んだ一覧と位置は役割ごとに残す。アーティストの詳細は `view` の表示で開く。
 */
const props = defineProps<{ role: string, view: ArtistView, emptyIcon: Component, emptyTitle: string }>()

const subsonic = useSubsonic()

// getArtists は全件を一度に返すので、一回で読み終える
const { items, done, loading, error, loadMore } = useInfiniteList<ArtistIndex>(`artists:${props.role}`, async () => {
  const res = await subsonic<{ artists: { index?: ArtistIndex[] } }>('getArtists', { role: props.role })
  return res.artists.index ?? []
}, Number.MAX_SAFE_INTEGER)
</script>

<template>
  <div>
    <ArtistList :index="items" :view="view" />

    <p v-if="loading" class="py-6 text-center text-body-sm text-fg-subtle">
      読み込み中…
    </p>
    <div v-else-if="error" class="flex flex-col items-center gap-3 py-6">
      <p class="text-body text-danger">
        読み込めませんでした
      </p>
      <Button variant="secondary" @click="loadMore">
        もう一度読み込む
      </Button>
    </div>
    <EmptyState v-else-if="done && items.length === 0" :icon="emptyIcon" :title="emptyTitle" />
  </div>
</template>
