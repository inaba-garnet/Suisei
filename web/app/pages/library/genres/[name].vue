<script setup lang="ts">
import { trackSortOptions } from '~/utils/tracks'

const route = useRoute()
const name = String(route.params.name)

useHead({ title: name })

// そのジャンルの曲を、トラックの一覧と同じ並び順で読み足す（docs/web.md の「一覧」）
const sort = useTrackSort('suisei-genre-song-sort')
</script>

<template>
  <div>
    <PageHeader :title="name" :back="{ to: '/library/genres' }">
      <div class="flex justify-end">
        <SortSelect v-model="sort" :options="trackSortOptions" />
      </div>
    </PageHeader>

    <TrackBrowser :key="sort" :sort="sort" :genre="name" />
  </div>
</template>
