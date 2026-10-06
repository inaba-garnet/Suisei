<script setup lang="ts">
import type { SearchResult } from '~/utils/search'
import type { Song } from '~/utils/subsonic'
import { watchDebounced } from '@vueuse/core'
import { formatDuration } from '~/utils/subsonic'

/** 未対応の Spotify の曲に対応させるローカルの曲を、検索して選ぶ（docs/web.md の「Spotify」）。 */
const props = defineProps<{ term: string, disabled?: boolean }>()
const emit = defineEmits<{ pick: [song: Song] }>()

const subsonic = useSubsonic()
const text = ref(props.term)
const songs = ref<Song[]>()
const error = ref(false)
let latest = 0

async function search(value: string) {
  const id = ++latest
  const q = value.trim()
  if (!q) {
    songs.value = []
    return
  }
  error.value = false
  try {
    const res = await subsonic<{ searchResult3: SearchResult }>('search3', {
      query: q,
      artistCount: 0,
      albumCount: 0,
      songCount: 10,
    })
    if (id === latest) {
      songs.value = res.searchResult3.song ?? []
    }
  }
  catch {
    if (id === latest) {
      error.value = true
    }
  }
}
search(text.value)
watchDebounced(text, search, { debounce: 300 })
</script>

<template>
  <div class="flex flex-col gap-2 pt-2">
    <Input v-model="text" type="search" aria-label="ライブラリの曲を検索" autocomplete="off" />
    <p v-if="error" class="text-body-sm text-danger">
      検索できませんでした
    </p>
    <p v-else-if="songs && songs.length === 0" class="text-body-sm text-fg-subtle">
      見つかりませんでした
    </p>
    <ul v-else-if="songs" class="flex flex-col" aria-label="検索の結果">
      <li v-for="song in songs" :key="song.id">
        <button
          type="button"
          class="flex w-full items-center gap-3 rounded-md px-2 py-2 text-left transition-colors hover:bg-surface-2 disabled:opacity-40"
          :disabled="disabled"
          @click="emit('pick', song)"
        >
          <span class="min-w-0 flex-1">
            <span class="block truncate text-body">{{ song.title }}</span>
            <span class="block truncate text-body-sm text-fg-subtle">{{ [song.artist, song.album].filter(Boolean).join(' · ') }}</span>
          </span>
          <span class="text-body-sm text-fg-subtle tabular-nums">{{ formatDuration(song.duration) }}</span>
        </button>
      </li>
    </ul>
  </div>
</template>
