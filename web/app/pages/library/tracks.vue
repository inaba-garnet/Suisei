<script setup lang="ts">
import { trackSortOptions } from '~/utils/tracks'

useHead({ title: 'トラック' })

const sort = useTrackSort('suisei-track-sort')
</script>

<template>
  <div>
    <PageHeader title="トラック" :back="{ to: '/library', label: 'ライブラリ', mobileOnly: true }">
      <div class="flex justify-end">
        <SortSelect v-model="sort" :options="trackSortOptions" />
      </div>
    </PageHeader>

    <TrackBrowser :key="sort" :sort="sort" />
  </div>
</template>
