<script setup lang="ts">
import type { ArtistIndex } from '~/utils/subsonic'
import { MicVocal } from 'lucide-vue-next'

useHead({ title: 'アーティスト' })

const subsonic = useSubsonic()

// getArtists は全件を一度に返すので、一回で読み終える
const { items, done, loading, error, loadMore } = useInfiniteList<ArtistIndex>('artists', async () => {
  const res = await subsonic<{ artists: { index?: ArtistIndex[] } }>('getArtists')
  return res.artists.index ?? []
}, Number.MAX_SAFE_INTEGER)
</script>

<template>
  <div>
    <PageHeader title="アーティスト" :back="{ to: '/library', label: 'ライブラリ', mobileOnly: true }" />

    <ArtistList :index="items" />

    <p v-if="loading" class="py-6 text-center text-body-sm text-fg-subtle">
      読み込み中…
    </p>
    <div v-else-if="error" class="flex flex-col items-center gap-3 py-6">
      <p class="text-body text-danger">
        アーティストを読み込めませんでした
      </p>
      <Button variant="secondary" @click="loadMore">
        もう一度読み込む
      </Button>
    </div>
    <EmptyState v-else-if="done && items.length === 0" :icon="MicVocal" title="アーティストがいません" />
  </div>
</template>
