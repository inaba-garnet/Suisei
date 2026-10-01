<script setup lang="ts">
import { favoriteSort, favoriteSortOptions } from '~/utils/favorites'

useHead({ title: 'お気に入り' })

const sort = useStoredSort('suisei-favorite-sort', favoriteSort)
</script>

<template>
  <div>
    <PageHeader title="お気に入り" :back="{ to: '/library', label: 'ライブラリ', mobileOnly: true }">
      <div class="flex justify-end">
        <SortSelect v-model="sort" :options="favoriteSortOptions" />
      </div>
    </PageHeader>

    <FavoriteSongs :key="sort" :sort="sort" />
  </div>
</template>
