<script setup lang="ts">
import type { AlbumWithSongs, Song } from '~/utils/subsonic'
import { ChevronLeft } from 'lucide-vue-next'
import { formatDuration } from '~/utils/subsonic'

const route = useRoute()
const router = useRouter()
const subsonic = useSubsonic()
const id = computed(() => String(route.params.id))

const { data: album, status, refresh } = useAsyncData(
  () => `album:${id.value}`,
  async () => (await subsonic<{ album: AlbumWithSongs }>('getAlbum', { id: id.value })).album,
)

useHead({ title: () => album.value?.name ?? 'アルバム' })

const songs = computed(() => album.value?.song ?? [])
const multiDisc = computed(() => new Set(songs.value.map(song => song.discNumber ?? 1)).size > 1)
const details = computed(() => {
  const a = album.value
  if (!a) {
    return ''
  }
  return [
    a.year,
    a.genres?.map(genre => genre.name).join('、'),
    `${a.songCount} 曲`,
    formatDuration(a.duration),
  ].filter(Boolean).join(' · ')
})

/** 前の曲とディスクが変わるところで見出しを出す。 */
function discHeading(index: number): number | undefined {
  if (!multiDisc.value) {
    return undefined
  }
  const disc = songs.value[index]?.discNumber ?? 1
  const previous = index > 0 ? songs.value[index - 1]?.discNumber ?? 1 : undefined
  return disc !== previous ? disc : undefined
}

/** 曲のアーティストがアルバムと違うときだけ出す。 */
function songArtist(song: Song): string | undefined {
  return song.artist && song.artist !== album.value?.artist ? song.artist : undefined
}

// 一覧から来たなら「戻る」で戻し、一覧の位置を戻す。直接開いたなら一覧を開く
function back() {
  if (window.history.state?.back) {
    router.back()
  }
  else {
    navigateTo('/library/albums')
  }
}
</script>

<template>
  <div>
    <button type="button" class="-ml-1 mb-3 inline-flex items-center gap-1 text-body-sm text-fg-muted hover:text-fg" @click="back">
      <ChevronLeft class="size-4" />
      アルバム
    </button>

    <p v-if="status === 'pending' && !album" class="py-6 text-center text-body-sm text-fg-subtle">
      読み込み中…
    </p>
    <div v-else-if="!album" class="flex flex-col items-center gap-3 py-6">
      <p class="text-body text-danger">
        アルバムを読み込めませんでした
      </p>
      <Button variant="secondary" @click="refresh()">
        もう一度読み込む
      </Button>
    </div>
    <template v-else>
      <header class="mb-6 flex flex-col gap-4 sm:flex-row sm:items-end">
        <CoverArt :id="album.coverArt" :size="200" :alt="album.name" class="w-40 shrink-0 sm:w-48" />
        <div class="flex min-w-0 flex-col gap-1">
          <h1 class="text-h1 font-semibold">
            {{ album.name }}
          </h1>
          <p class="text-body-lg text-fg-muted">
            {{ album.artist }}
          </p>
          <p class="text-body-sm text-fg-subtle">
            {{ details }}
          </p>
        </div>
      </header>

      <ol data-testid="songs">
        <template v-for="(song, index) in songs" :key="song.id">
          <li v-if="discHeading(index)" class="border-b border-divider px-3 pt-4 pb-2 text-caption font-medium text-fg-subtle">
            ディスク {{ discHeading(index) }}
          </li>
          <li class="flex h-12 items-center gap-3 border-b border-divider px-3">
            <span class="w-6 shrink-0 text-right text-body-sm text-fg-subtle tabular-nums">{{ song.track ?? '' }}</span>
            <span class="min-w-0 flex-1">
              <span class="block truncate text-body">{{ song.title }}</span>
              <span v-if="songArtist(song)" class="block truncate text-body-sm text-fg-subtle">{{ songArtist(song) }}</span>
            </span>
            <span class="shrink-0 text-body-sm text-fg-subtle tabular-nums">{{ formatDuration(song.duration) }}</span>
          </li>
        </template>
      </ol>
    </template>
  </div>
</template>
