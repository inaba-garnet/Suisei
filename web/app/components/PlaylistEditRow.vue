<script setup lang="ts">
import type { Song } from '~/utils/subsonic'
import { GripVertical, X } from 'lucide-vue-next'

/**
 * 編集中のプレイリストの一行（docs/web.md の「プレイリスト」）。キューの行と同じく、右端の取っ手で並べ替え、
 * PC は × のボタン、スマホは左に払って外す。
 */
const props = defineProps<{ song: Song, swipe: boolean }>()
const emit = defineEmits<{ remove: [] }>()

const { swiping, handlers } = useSwipeRemove(() => props.swipe, () => emit('remove'))
</script>

<template>
  <li class="relative overflow-hidden border-b border-divider">
    <div
      v-if="swiping?.active"
      aria-hidden="true"
      class="absolute inset-0 flex items-center justify-end bg-danger px-4 text-body-sm font-medium text-white"
    >
      削除
    </div>
    <div
      class="relative flex h-14 items-center gap-3 px-3"
      :class="swiping?.active ? 'bg-surface-0 md:bg-surface-1' : 'transition-transform duration-200'"
      :style="swiping?.active ? { transform: `translateX(${swiping.dx}px)` } : undefined"
      v-on="handlers"
    >
      <button
        v-if="!swipe"
        type="button"
        :aria-label="`${song.title}をプレイリストから外す`"
        class="flex size-8 shrink-0 items-center justify-center rounded-full text-fg-subtle transition-colors hover:bg-surface-2 hover:text-fg"
        @click="emit('remove')"
      >
        <X class="size-4" />
      </button>
      <CoverArt :id="song.coverArt" :size="40" :alt="song.album" class="size-10 shrink-0" />
      <span class="min-w-0 flex-1">
        <span class="block truncate text-body" data-testid="playlist-edit-title">{{ song.title }}</span>
        <span class="block truncate text-body-sm text-fg-subtle">{{ [song.artist, song.album].filter(Boolean).join(' · ') }}</span>
      </span>
      <span
        :aria-label="`${song.title}を並べ替える`"
        role="img"
        class="flex size-8 shrink-0 cursor-grab touch-none items-center justify-center text-fg-subtle active:cursor-grabbing"
        data-playlist-handle
        data-swipe-ignore
      >
        <GripVertical class="size-4" />
      </span>
    </div>
  </li>
</template>
