<script setup lang="ts">
import { Feather } from 'lucide-vue-next'

useHead({ title: '作曲家' })

const favorite = useListOption('composers-favorite', false)
</script>

<template>
  <div>
    <PageHeader title="作曲家" :back="{ to: '/library', label: 'ライブラリ', mobileOnly: true }">
      <FavoriteToggle v-model="favorite" />
    </PageHeader>

    <!-- 作詞と編曲も、作曲と一緒にまとめる（docs/web.md の「一覧」） -->
    <ArtistBrowser role="composer,lyricist,arranger" view="composer" :favorite="favorite" :empty-icon="Feather" empty-title="作曲家がいません" />
  </div>
</template>
