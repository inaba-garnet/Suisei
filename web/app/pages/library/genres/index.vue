<script setup lang="ts">
import type { Genre } from '~/utils/subsonic'
import { useLocalStorage } from '@vueuse/core'
import { Shapes } from 'lucide-vue-next'
import { genreHues, genreSort, genreSortOptions, sortGenres } from '~/utils/genres'

useHead({ title: 'ジャンル' })

const subsonic = useSubsonic()

// getGenres は全件を一度に返すので、一回で読み終える
const { items, done, loading, error, loadMore } = useInfiniteList<Genre>('genres', async () => {
  const res = await subsonic<{ genres: { genre?: Genre[] } }>('getGenres')
  return res.genres.genre ?? []
}, Number.MAX_SAFE_INTEGER)

// 選んだ並び順は残す（docs/web.md の「一覧」）
const stored = useLocalStorage('suisei-genre-sort', 'name')
const sort = computed({
  get: () => genreSort(stored.value),
  set: (value) => {
    stored.value = value
  },
})
const genres = computed(() => sortGenres(items.value, sort.value))
const hues = computed(() => genreHues(items.value))
</script>

<template>
  <div>
    <PageHeader title="ジャンル" :back="{ to: '/library', label: 'ライブラリ', mobileOnly: true }">
      <div class="flex justify-end">
        <SortSelect v-model="sort" :options="genreSortOptions" />
      </div>
    </PageHeader>

    <ul class="grid grid-cols-2 gap-2 md:grid-cols-3 4xl:grid-cols-4" data-testid="genres">
      <li v-for="genre in genres" :key="genre.value">
        <NuxtLink
          :to="`/library/genres/${encodeURIComponent(genre.value)}`"
          class="flex h-18 flex-col justify-center overflow-hidden rounded-lg px-3 text-white transition-[filter] hover:brightness-110"
          :style="{ background: `linear-gradient(135deg, hsl(${hues.get(genre.value)} 55% 45%), hsl(${hues.get(genre.value)! + 25} 60% 28%))` }"
        >
          <span class="truncate text-body font-semibold">{{ genre.value }}</span>
          <span class="truncate text-body-sm text-white/80">{{ genre.songCount }} 曲 · {{ genre.albumCount }} 枚</span>
        </NuxtLink>
      </li>
    </ul>

    <p v-if="loading" class="py-6 text-center text-body-sm text-fg-subtle">
      読み込み中…
    </p>
    <div v-else-if="error" class="flex flex-col items-center gap-3 py-6">
      <p class="text-body text-danger">
        ジャンルを読み込めませんでした
      </p>
      <Button variant="secondary" @click="loadMore">
        もう一度読み込む
      </Button>
    </div>
    <EmptyState v-else-if="done && items.length === 0" :icon="Shapes" title="ジャンルがありません" />
  </div>
</template>
