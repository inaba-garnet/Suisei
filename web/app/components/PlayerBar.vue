<script setup lang="ts">
import { useMediaQuery } from '@vueuse/core'
import { SkipBack, SkipForward } from 'lucide-vue-next'
import { formatDuration } from '~/utils/subsonic'

/** 再生バー。スマホではタブの上の小さな帯、PC では下端いっぱいに出す（docs/web.md の「再生」）。 */
const { state, current, canPrevious, canNext, previous, next } = usePlayer()
const sheet = usePlayerSheet()
// PC（md 以上）では開かないので、ボタンにしない
const wide = useMediaQuery('(min-width: 48rem)')
</script>

<template>
  <div
    class="relative mx-2 mb-2 flex items-center gap-3 rounded-lg border border-border-subtle bg-surface-1 px-3 py-2 md:m-0 md:h-full md:gap-4 md:rounded-none md:border-0 md:bg-transparent md:p-0"
    data-testid="player-bar"
  >
    <!-- スマホでは、ジャケットと曲名を押すと下から再生画面を引き出す。PC は右の欄か下端のバーで足りるので開かない -->
    <component
      :is="wide ? 'div' : 'button'"
      class="flex min-w-0 flex-1 items-center gap-3 text-left md:gap-4"
      v-bind="wide ? {} : { 'type': 'button', 'disabled': !current, 'aria-label': '再生画面を開く' }"
      @click="wide || (sheet = true)"
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
    </component>
    <span v-if="current" class="hidden shrink-0 text-body-sm text-fg-subtle tabular-nums md:inline">
      {{ formatDuration(state.position) }} / {{ formatDuration(current.duration) }}
    </span>
    <div class="flex items-center gap-1.5">
      <PlayerModes mode="shuffle" class="hidden md:inline-flex" />
      <Button variant="ghost" size="icon" class="hidden md:inline-flex" :disabled="!canPrevious" aria-label="前の曲" @click="previous">
        <SkipBack class="size-4" />
      </Button>
      <PlayerToggle />
      <Button variant="ghost" size="icon" class="hidden md:inline-flex" :disabled="!canNext" aria-label="次の曲" @click="next">
        <SkipForward class="size-4" />
      </Button>
      <PlayerModes mode="repeat" class="hidden md:inline-flex" />
    </div>
    <PlayerVolume class="hidden md:flex" />
    <PlayerLyricsButton class="hidden md:inline-flex" />
    <PlayerQueueButton class="hidden md:inline-flex" />
    <!-- スマホは帯の下端に表示だけ、PC は再生バーの上の区切り線に重ねて、つまんで動かせるようにする -->
    <PlayerProgress v-if="current" class="absolute inset-x-3 bottom-0 h-0.5 md:hidden" />
    <PlayerSeek v-if="current" class="absolute -inset-x-4 -top-[17px] hidden md:flex" />
  </div>
</template>
