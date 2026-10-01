<script setup lang="ts">
import type { ArtistView } from '~/utils/artist'
import { MicVocal } from 'lucide-vue-next'

useHead({ title: 'アーティスト' })

// アルバムアーティストか、曲のアーティスト全員か。開き直したときはアルバムアーティストに戻す（docs/web.md の「一覧」）
type Role = 'albumartist' | 'artist'
const role = useListOption<Role>('artists-role', 'albumartist')
const favorite = useListOption('artists-favorite', false)

// アルバムアーティストはアルバムだけを、曲のアーティストは曲を詳細に出す
const views: Record<Role, ArtistView> = { albumartist: 'albums', artist: 'tracks' }

const roles: { value: Role, label: string }[] = [
  { value: 'albumartist', label: 'アルバムアーティスト' },
  { value: 'artist', label: 'アーティスト' },
]
</script>

<template>
  <div>
    <PageHeader title="アーティスト" :back="{ to: '/library', label: 'ライブラリ', mobileOnly: true }">
      <div class="flex flex-wrap items-center gap-2">
        <SegmentedControl v-model="role" label="表示するアーティスト" :options="roles" />
        <FavoriteToggle v-model="favorite" />
      </div>
    </PageHeader>

    <ArtistBrowser :key="role" :role="role" :view="views[role]" :favorite="favorite" :empty-icon="MicVocal" empty-title="アーティストがいません" />
  </div>
</template>
