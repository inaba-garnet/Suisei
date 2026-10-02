<script setup lang="ts">
import { SkipBack, SkipForward } from 'lucide-vue-next'
import { formatDuration } from '~/utils/subsonic'

/** 再生バー。スマホではタブの上の小さな帯、PC では下端いっぱいに出す（docs/web.md の「再生」）。 */
const { state, current, canPrevious, canNext, previous, next } = usePlayer()
</script>

<template>
  <div
    class="relative mx-2 mb-2 flex items-center gap-3 rounded-lg border border-border-subtle bg-surface-1 px-3 py-2 md:m-0 md:h-full md:gap-4 md:rounded-none md:border-0 md:bg-transparent md:p-0"
    data-testid="player-bar"
  >
    <CoverArt :id="current?.coverArt" :size="44" :alt="current?.album" class="size-11 shrink-0 rounded-md" />
    <div class="min-w-0 flex-1">
      <template v-if="current">
        <p class="truncate text-body">
          {{ current.title }}
        </p>
        <p v-if="state.failed" class="truncate text-body-sm text-danger">
          再生できませんでした
        </p>
        <p v-else class="truncate text-body-sm text-fg-subtle">
          {{ current.artist }}
        </p>
      </template>
      <p v-else class="truncate text-body text-fg-muted">
        再生していません
      </p>
    </div>
    <span v-if="current" class="hidden shrink-0 text-body-sm text-fg-subtle tabular-nums md:inline">
      {{ formatDuration(state.position) }} / {{ formatDuration(current.duration) }}
    </span>
    <div class="flex items-center gap-1.5">
      <Button variant="ghost" size="icon" class="hidden md:inline-flex" :disabled="!canPrevious" aria-label="前の曲" @click="previous">
        <SkipBack class="size-4" />
      </Button>
      <PlayerToggle />
      <Button variant="ghost" size="icon" class="hidden md:inline-flex" :disabled="!canNext" aria-label="次の曲" @click="next">
        <SkipForward class="size-4" />
      </Button>
    </div>
    <!-- スマホは帯の下端、PC は再生バーの上の区切り線に重ねる -->
    <PlayerProgress v-if="current" class="absolute inset-x-3 bottom-0 h-0.5 md:-inset-x-4 md:-top-2.5 md:bottom-auto" />
  </div>
</template>
