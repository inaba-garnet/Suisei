<script setup lang="ts">
import { useLocalStorage } from '@vueuse/core'
import { trackSort, trackSortOptions } from '~/utils/tracks'

useHead({ title: 'トラック' })

// 選んだ並び順は残す（docs/web.md の「一覧」）
const stored = useLocalStorage('suisei-track-sort', 'title')
const sort = computed({
  get: () => trackSort(stored.value),
  set: (value) => {
    stored.value = value
  },
})

// 切り替えたときは、残した位置を使わずに先頭から出す
const popped = usePopped()
const scroller = useScroller()
watch(sort, () => {
  popped.value = false
  if (scroller.value) {
    scroller.value.scrollTop = 0
  }
})
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
