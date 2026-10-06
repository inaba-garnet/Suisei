<script setup lang="ts">
import type { PlaylistRow } from '~/utils/playlists'
import type { PlaylistWithSongs } from '~/utils/subsonic'
import { useMediaQuery } from '@vueuse/core'
import { ListMusic, Pencil, Play } from 'lucide-vue-next'
import Sortable from 'sortablejs'
import { playlistUpdate } from '~/utils/playlists'
import { coverArtUrl, formatDuration } from '~/utils/subsonic'

/**
 * プレイリストの詳細（docs/web.md の「プレイリスト」）。アルバムの詳細に寄せ、ジャケットの背景と見出しの下に曲を並べる。
 * 「編集」で名前と曲の並びを手元で変え、「完了」でまとめて保存する。
 */
const route = useRoute()
const subsonic = useSubsonic()
const { start } = usePlayer()
const id = computed(() => String(route.params.id))

const { data: playlist, status, refresh } = useAsyncData(
  () => `playlist:${id.value}`,
  async () => (await subsonic<{ playlist: PlaylistWithSongs }>('getPlaylist', { id: id.value })).playlist,
)

useHead({ title: () => playlist.value?.name ?? 'プレイリスト' })

const songs = computed(() => playlist.value?.entry ?? [])
const details = computed(() => playlist.value ? `${playlist.value.songCount} 曲 · ${formatDuration(playlist.value.duration)}` : '')

// 大きな見出しが帯の下に隠れたら、帯に名前を出す
const heading = ref<HTMLElement>()
const cover = ref()
const { titleVisible, bandOpacity, coverOpacity } = useDetailHeader(heading, cover)

// 編集。手元の写しを変え、「完了」でまとめて保存する。「キャンセル」なら写しを捨てる
const editing = ref(false)
const name = ref('')
const rows = ref<PlaylistRow[]>([])
const saving = ref(false)
const saveFailed = ref(false)
const confirmDelete = ref(false)
// スマホは行を左に払って外し、PC は × のボタンで外す
const wide = useMediaQuery('(min-width: 48rem)')

function startEdit() {
  name.value = playlist.value?.name ?? ''
  rows.value = songs.value.map((song, key) => ({ key, song }))
  saveFailed.value = false
  confirmDelete.value = false
  editing.value = true
}

async function save() {
  const p = playlist.value
  if (!p || saving.value) {
    return
  }
  const params = playlistUpdate(p.id, name.value, { name: p.name, songs: songs.value }, rows.value)
  if (!params) {
    editing.value = false
    return
  }
  saving.value = true
  saveFailed.value = false
  try {
    // 曲の ID が多いと URL に収まらないので、フォームで送る
    await subsonic('updatePlaylist', params, { post: true })
    await refresh()
    editing.value = false
  }
  catch {
    // 編集を続けたまま知らせ、もう一度「完了」を押せるようにする
    saveFailed.value = true
  }
  finally {
    saving.value = false
  }
}

async function remove() {
  const p = playlist.value
  if (!p || saving.value) {
    return
  }
  saving.value = true
  saveFailed.value = false
  try {
    await subsonic('deletePlaylist', { id: p.id })
    await navigateTo('/playlists', { replace: true })
  }
  catch {
    saveFailed.value = true
    saving.value = false
  }
}

// 並べ替えは取っ手でだけ引かせる。行の並べ方は Vue に任せるので、SortableJS が動かした行は元の位置に戻してから写しを並べ替える
const list = ref<HTMLElement>()
let sortable: Sortable | undefined
let nextSibling: Node | null = null
watch(list, (el) => {
  sortable?.destroy()
  sortable = el
    ? Sortable.create(el, {
        handle: '[data-playlist-handle]',
        forceFallback: true,
        fallbackOnBody: true,
        onStart: ({ item }) => {
          nextSibling = item.nextSibling
        },
        onEnd: ({ item, from, oldIndex, newIndex }) => {
          if (oldIndex === undefined || newIndex === undefined || oldIndex === newIndex) {
            return
          }
          from.insertBefore(item, nextSibling)
          const next = [...rows.value]
          const [moved] = next.splice(oldIndex, 1)
          next.splice(newIndex, 0, moved!)
          rows.value = next
        },
      })
    : undefined
})
onBeforeUnmount(() => sortable?.destroy())
</script>

<template>
  <div>
    <div
      v-if="playlist?.coverArt"
      aria-hidden="true"
      class="pointer-events-none absolute inset-x-0 top-0 h-[30rem] overflow-hidden md:h-96"
      data-testid="playlist-hero"
    >
      <img :src="coverArtUrl(playlist.coverArt, 300)" alt="" class="size-full scale-125 object-cover opacity-90 blur-xl md:opacity-50 md:blur-2xl">
      <div class="absolute inset-0 bg-gradient-to-b from-surface-0/20 via-surface-0/55 to-surface-0 md:from-surface-1/30 md:via-surface-1/65 md:to-surface-1" />
    </div>

    <PageHeader
      :title="playlist?.name ?? 'プレイリスト'"
      :back="{ to: '/playlists' }"
      detail
      overlay
      :title-visible="titleVisible"
      :cover="playlist?.coverArt"
      :band-opacity="bandOpacity"
    />

    <p v-if="status === 'pending' && !playlist" class="relative py-6 text-center text-body-sm text-fg-subtle">
      読み込み中…
    </p>
    <div v-else-if="!playlist" class="relative flex flex-col items-center gap-3 py-6">
      <p class="text-body text-danger">
        プレイリストを読み込めませんでした
      </p>
      <Button variant="secondary" @click="refresh()">
        もう一度読み込む
      </Button>
    </div>
    <div v-else class="relative">
      <header class="mb-8 flex flex-col gap-5 sm:flex-row sm:items-end">
        <CoverArt :id="playlist.coverArt" ref="cover" :size="240" :style="{ opacity: coverOpacity }" :alt="playlist.name" class="w-44 shrink-0 shadow-[0_8px_32px_rgb(0_0_0/0.35)] sm:w-56" />
        <div class="flex min-w-0 flex-1 flex-col gap-1.5">
          <template v-if="editing">
            <label>
              <span class="sr-only">プレイリストの名前</span>
              <Input v-model="name" autocomplete="off" enterkeyhint="done" @keydown.enter.prevent="save" />
            </label>
          </template>
          <h1 v-else ref="heading" class="line-clamp-2 text-display font-semibold break-words" :title="playlist.name">
            {{ playlist.name }}
          </h1>
          <p class="text-body-sm text-fg-subtle">
            {{ details }}
          </p>
          <div class="mt-3 flex flex-wrap items-center gap-3">
            <template v-if="editing">
              <Button size="lg" :disabled="saving || !name.trim()" @click="save">
                完了
              </Button>
              <Button size="lg" variant="ghost" :disabled="saving" @click="editing = false">
                キャンセル
              </Button>
            </template>
            <template v-else>
              <Button size="lg" :disabled="!songs.length" @click="start(songs)">
                <Play class="size-4" />
                すべて再生
              </Button>
              <Button v-if="!playlist?.readonly" size="lg" variant="secondary" @click="startEdit">
                <Pencil class="size-4" />
                編集
              </Button>
            </template>
          </div>
          <p v-if="saveFailed" class="text-body-sm text-danger">
            保存できませんでした
          </p>
        </div>
      </header>

      <template v-if="editing">
        <ol ref="list" data-testid="playlist-edit">
          <PlaylistEditRow
            v-for="row in rows"
            :key="row.key"
            :song="row.song"
            :swipe="!wide"
            @remove="rows = rows.filter(r => r.key !== row.key)"
          />
        </ol>
        <div class="mt-8 flex flex-wrap items-center gap-3">
          <Button v-if="!confirmDelete" variant="ghost" class="text-danger" :disabled="saving" @click="confirmDelete = true">
            プレイリストを削除
          </Button>
          <template v-else>
            <span class="text-body-sm text-fg-muted">「{{ playlist.name }}」を削除しますか？</span>
            <Button variant="destructive" :disabled="saving" @click="remove">
              削除する
            </Button>
            <Button variant="ghost" :disabled="saving" @click="confirmDelete = false">
              やめる
            </Button>
          </template>
        </div>
      </template>
      <EmptyState v-else-if="!songs.length" :icon="ListMusic" title="曲がありません" />
      <SongList v-else :songs="songs" :more="false" />
    </div>
  </div>
</template>
