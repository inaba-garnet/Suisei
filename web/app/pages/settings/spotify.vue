<script setup lang="ts">
import type { SpotifyTrack } from '~/utils/spotify'
import type { Song } from '~/utils/subsonic'
import { CircleCheck } from 'lucide-vue-next'
import { searchTermOf } from '~/utils/spotify'
import { formatDuration } from '~/utils/subsonic'

useHead({ title: '未対応の曲' })

const spotify = useSpotify()
const toast = useToast()

const tracks = ref<SpotifyTrack[]>()
const error = ref(false)
/** 検索欄を開いている行 */
const open = ref<string>()
const busy = ref(false)

async function load() {
  error.value = false
  try {
    tracks.value = (await spotify.tracks(false)).tracks
  }
  catch {
    error.value = true
  }
}
onMounted(load)

async function pick(track: SpotifyTrack, song: Song) {
  busy.value = true
  try {
    await spotify.link(track.id, song.id)
    tracks.value = tracks.value?.filter(t => t.id !== track.id)
    open.value = undefined
    toast.show(`「${song.title}」をお気に入りにしました`)
  }
  catch {
    toast.show('対応を付けられませんでした')
  }
  finally {
    busy.value = false
  }
}

async function ignore(track: SpotifyTrack) {
  busy.value = true
  try {
    await spotify.link(track.id, null)
    track.method = 'ignored'
    open.value = undefined
  }
  catch {
    toast.show('変更できませんでした')
  }
  finally {
    busy.value = false
  }
}
</script>

<template>
  <div>
    <PageHeader title="未対応の曲" :back="{ to: '/settings', label: '設定' }" />

    <p class="mb-4 text-body-sm text-fg-subtle">
      Spotify のお気に入りのうち、ライブラリの曲と対応できなかった曲です。ライブラリから選ぶと、その曲をお気に入りにします。
    </p>

    <p v-if="error" class="py-6 text-center text-body text-danger">
      読み込めませんでした
    </p>
    <p v-else-if="!tracks" class="py-6 text-center text-body-sm text-fg-subtle">
      読み込み中…
    </p>
    <EmptyState v-else-if="tracks.length === 0" :icon="CircleCheck" title="未対応の曲はありません" />
    <ul v-else class="flex flex-col" data-testid="spotify-unmatched-list">
      <li v-for="track in tracks" :key="track.id" class="border-b border-divider py-3" :data-spotify-id="track.id">
        <div class="flex items-center gap-3">
          <div class="min-w-0 flex-1" :class="{ 'opacity-60': track.method === 'ignored' }">
            <p class="truncate text-body">
              {{ track.title }}
            </p>
            <p class="truncate text-body-sm text-fg-subtle">
              {{ [track.artists.join(' / '), track.album].filter(Boolean).join(' · ') }}
            </p>
          </div>
          <span v-if="track.method === 'ignored'" class="text-body-sm text-fg-subtle">対応させない</span>
          <span class="text-body-sm text-fg-subtle tabular-nums">{{ formatDuration(track.durationMs / 1000) }}</span>
        </div>
        <div class="mt-2 flex gap-2">
          <Button size="sm" variant="secondary" :disabled="busy" @click="open = open === track.id ? undefined : track.id">
            {{ open === track.id ? '閉じる' : '選ぶ' }}
          </Button>
          <Button v-if="track.method !== 'ignored'" size="sm" variant="ghost" :disabled="busy" @click="ignore(track)">
            対応させない
          </Button>
        </div>
        <SpotifyMatchPicker v-if="open === track.id" :term="searchTermOf(track)" :disabled="busy" @pick="song => pick(track, song)" />
      </li>
    </ul>
  </div>
</template>
