<script setup lang="ts">
import type { Component } from 'vue'
import type { Playlist } from '~/utils/subsonic'
import { onKeyStroke, useEventListener, useMediaQuery } from '@vueuse/core'
import { ChevronLeft, Disc3, ListEnd, ListPlus, ListStart, MicVocal, Plus } from 'lucide-vue-next'

/**
 * 曲とアルバムの「…」のメニュー（docs/web.md の「メニュー」）。layout に一つだけ置く。
 * PC は「…」のボタンの横に小さく出し、スマホは下から出すシートにする。
 */
const { state, close } = useSongMenu()
const { playNext, addToQueue } = usePlayer()
const { show } = useToast()
const subsonic = useSubsonic()
const route = useRoute()
const wide = useMediaQuery('(min-width: 48rem)')

const target = computed(() => state.value?.target)
const songs = computed(() => {
  const t = target.value
  return t ? (t.kind === 'song' ? [t.song] : [...t.songs]) : []
})
const title = computed(() => {
  const t = target.value
  return t?.kind === 'song' ? t.song.title : t?.album.name ?? ''
})
const subtitle = computed(() => {
  const t = target.value
  return t?.kind === 'song' ? [t.song.artist, t.song.album].filter(Boolean).join(' · ') : t?.album.artist ?? ''
})
const coverArt = computed(() => {
  const t = target.value
  return t?.kind === 'song' ? t.song.coverArt : t?.album.coverArt
})

interface Item { label: string, icon: Component, action: () => void }

/** 移る先。いま開いている画面と同じ先は出さない。 */
const links = computed<Item[]>(() => {
  const t = target.value
  if (!t) {
    return []
  }
  const items: Item[] = []
  if (t.kind === 'song' && t.song.albumId && route.path !== `/library/albums/${t.song.albumId}`) {
    const to = `/library/albums/${t.song.albumId}`
    items.push({ label: 'アルバムへ移動', icon: Disc3, action: () => go(to) })
  }
  // 曲のアーティストは、曲で参加した人として見たいので曲のアーティストの表示で開く。アルバムはアルバムアーティストの表示
  const view = t.kind === 'song' ? 'tracks' : 'albums'
  const artists = (t.kind === 'song' ? t.song.artists : t.album.artists) ?? []
  for (const artist of artists) {
    if (route.path === `/library/artists/${artist.id}`) {
      continue
    }
    const label = artists.length > 1 ? `${artist.name}へ移動` : 'アーティストへ移動'
    items.push({ label, icon: MicVocal, action: () => go({ path: `/library/artists/${artist.id}`, query: { view } }) })
  }
  return items
})

const items = computed<Item[]>(() => [
  { label: '次に再生', icon: ListStart, action: () => run(() => playNext(songs.value), '次に再生します') },
  { label: 'キューの最後に追加', icon: ListEnd, action: () => run(() => addToQueue(songs.value), 'キューに追加しました') },
  { label: 'プレイリストに追加', icon: ListPlus, action: openPlaylists },
  ...links.value,
])

function run(action: () => void, message: string) {
  action()
  close()
  show(message)
}

function go(to: Parameters<typeof navigateTo>[0]) {
  close()
  navigateTo(to)
}

// プレイリストに足す。一覧を開いてから選ぶ
const view = ref<'main' | 'playlists'>('main')
const playlists = ref<Playlist[]>()
const loadFailed = ref(false)
const creating = ref(false)
const newName = ref('')
const busy = ref(false)
const failed = ref(false)

async function openPlaylists() {
  view.value = 'playlists'
  playlists.value = undefined
  loadFailed.value = false
  try {
    // お気に入りのプレイリストのように曲を足せないものは、選ばせない
    const all = (await subsonic<{ playlists: { playlist?: Playlist[] } }>('getPlaylists')).playlists.playlist ?? []
    playlists.value = all.filter(p => !p.readonly)
  }
  catch {
    loadFailed.value = true
  }
  await focusFirst()
}

async function addTo(playlist: Playlist) {
  await save(
    () => subsonic('updatePlaylist', { playlistId: playlist.id, songIdToAdd: songs.value.map(s => s.id) }, { post: true }),
    `「${playlist.name}」に追加しました`,
  )
}

async function create() {
  const name = newName.value.trim()
  if (!name) {
    return
  }
  await save(
    () => subsonic('createPlaylist', { name, songId: songs.value.map(s => s.id) }, { post: true }),
    `「${name}」を作って追加しました`,
  )
}

async function save(call: () => Promise<unknown>, message: string) {
  if (busy.value) {
    return
  }
  busy.value = true
  failed.value = false
  try {
    await call()
    close()
    show(message)
  }
  catch {
    failed.value = true
  }
  finally {
    busy.value = false
  }
}

// 開くたびに最初の画面に戻し、最初の項目にフォーカスを移す
const panel = ref<HTMLElement>()
const position = ref<Record<string, string>>({})
watch(state, async (s) => {
  view.value = 'main'
  creating.value = false
  newName.value = ''
  failed.value = false
  if (s) {
    await nextTick()
    place()
    await focusFirst()
  }
})

async function focusFirst() {
  await nextTick()
  panel.value?.querySelector<HTMLElement>('[role="menuitem"], input')?.focus()
}

/** PC は「…」のボタンの下（下が足りなければ上）に、右端をそろえて出す。 */
function place() {
  const anchor = state.value?.anchor
  const el = panel.value
  if (!wide.value || !anchor || !el) {
    position.value = {}
    return
  }
  const rect = anchor.getBoundingClientRect()
  const height = el.offsetHeight
  const below = rect.bottom + 4 + height <= window.innerHeight - 8
  position.value = {
    right: `${Math.max(8, window.innerWidth - rect.right)}px`,
    top: below ? `${rect.bottom + 4}px` : `${Math.max(8, rect.top - 4 - height)}px`,
  }
}
watch(view, () => nextTick(place))

// 画面を移ったり、PC で一覧をスクロールしたりしたら閉じる。ボタンから離れた位置に残らないようにするため
watch(() => route.fullPath, () => state.value && close())
const scroller = useScroller()
useEventListener(scroller, 'scroll', () => {
  if (state.value && wide.value) {
    close()
  }
}, { passive: true })

onKeyStroke('Escape', () => state.value && close())

/** 上下の矢印で項目を移る。 */
function onKeydown(e: KeyboardEvent) {
  if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') {
    return
  }
  const all = [...panel.value?.querySelectorAll<HTMLElement>('[role="menuitem"]') ?? []]
  const i = all.indexOf(document.activeElement as HTMLElement)
  const next = all[(i + (e.key === 'ArrowDown' ? 1 : -1) + all.length) % all.length]
  next?.focus()
  e.preventDefault()
}
</script>

<template>
  <Teleport to="body">
    <div v-if="state" class="fixed inset-0 z-40" data-testid="song-menu">
      <!-- 後ろを押すと閉じる。スマホは後ろを暗くしてシートに目を向ける -->
      <div class="absolute inset-0" :class="{ 'bg-black/50': !wide }" @click="close" />
      <div
        ref="panel"
        role="menu"
        :aria-label="`${title}のメニュー`"
        class="absolute flex flex-col overflow-y-auto border border-border-default bg-surface-2 shadow-elevation-3"
        :class="wide
          ? 'w-64 max-h-[min(28rem,calc(100dvh-16px))] rounded-lg p-1'
          : 'inset-x-0 bottom-0 max-h-[80dvh] rounded-t-xl px-2 pt-2 pb-[calc(env(safe-area-inset-bottom)+8px)]'"
        :style="position"
        @keydown="onKeydown"
      >
        <!-- スマホは、何に対するメニューかを上に出す -->
        <div v-if="!wide" class="mb-1 flex items-center gap-3 border-b border-divider px-2 pt-1 pb-3">
          <CoverArt :id="coverArt" :size="48" :alt="title" class="size-12 shrink-0" />
          <span class="min-w-0">
            <span class="block truncate text-body-lg font-medium">{{ title }}</span>
            <span class="block truncate text-body-sm text-fg-subtle">{{ subtitle }}</span>
          </span>
        </div>

        <template v-if="view === 'main'">
          <button
            v-for="item in items"
            :key="item.label"
            type="button"
            role="menuitem"
            class="item"
            @click="item.action"
          >
            <component :is="item.icon" class="size-4 shrink-0 text-fg-subtle" />
            {{ item.label }}
          </button>
        </template>

        <template v-else>
          <button type="button" role="menuitem" class="item text-fg-muted" @click="view = 'main'">
            <ChevronLeft class="size-4 shrink-0" />
            プレイリストに追加
          </button>
          <form v-if="creating" class="flex items-center gap-2 px-2 py-1.5" @submit.prevent="create">
            <label class="min-w-0 flex-1">
              <span class="sr-only">新しいプレイリストの名前</span>
              <Input v-model="newName" placeholder="プレイリストの名前" autocomplete="off" enterkeyhint="done" class="h-9" />
            </label>
            <Button type="submit" :disabled="!newName.trim() || busy">
              作成
            </Button>
          </form>
          <button v-else type="button" role="menuitem" class="item" @click="creating = true; focusFirst()">
            <Plus class="size-4 shrink-0 text-fg-subtle" />
            新規プレイリスト
          </button>
          <p v-if="loadFailed" class="px-3 py-2 text-body-sm text-danger">
            プレイリストを読み込めませんでした
          </p>
          <p v-else-if="!playlists" class="px-3 py-2 text-body-sm text-fg-subtle">
            読み込み中…
          </p>
          <button
            v-for="playlist in playlists"
            :key="playlist.id"
            type="button"
            role="menuitem"
            class="item"
            :disabled="busy"
            @click="addTo(playlist)"
          >
            <CoverArt :id="playlist.coverArt" :size="32" :alt="playlist.name" class="size-6 shrink-0" />
            <span class="min-w-0 flex-1 truncate">{{ playlist.name }}</span>
          </button>
          <p v-if="failed" class="px-3 py-2 text-body-sm text-danger">
            追加できませんでした
          </p>
        </template>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
@reference '~/assets/css/main.css';

.item {
  @apply flex min-h-10 w-full items-center gap-3 rounded-md px-3 text-left text-body text-fg transition-colors outline-none hover:bg-surface-3 focus-visible:bg-surface-3 disabled:opacity-40 md:min-h-8;
}
</style>
