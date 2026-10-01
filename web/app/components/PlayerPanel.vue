<script setup lang="ts">
import { Music, Play, Repeat, Shuffle, SkipBack, SkipForward } from 'lucide-vue-next'
</script>

<template>
  <!--
    広い画面の右の欄に置く再生プレイヤーの枠（docs/web.md の「画面構成」）。
    再生を入れるまでは、何も再生していない状態だけを出す。
  -->
  <section
    class="flex h-full flex-col gap-5 overflow-y-auto rounded-xl border border-divider bg-surface-1 p-5"
    aria-label="再生プレイヤー"
    data-testid="player-panel"
  >
    <h2 class="text-body font-semibold">
      再生中
    </h2>
    <div class="flex aspect-square items-center justify-center rounded-lg border border-border-subtle bg-surface-3 text-fg-subtle">
      <Music class="size-1/4" />
    </div>
    <p class="truncate text-body-lg text-fg-muted">
      再生していません
    </p>
    <div class="flex items-center justify-between">
      <Button variant="ghost" size="icon" disabled aria-label="シャッフル">
        <Shuffle class="size-4" />
      </Button>
      <Button variant="ghost" size="icon" disabled aria-label="前の曲">
        <SkipBack class="size-4" />
      </Button>
      <button
        type="button"
        class="flex size-12 items-center justify-center rounded-full border border-accent-base bg-accent-subtle text-fg shadow-[0_0_12px_var(--glow-medium)] disabled:pointer-events-none disabled:opacity-50"
        disabled
        aria-label="再生"
      >
        <Play class="size-5" />
      </button>
      <Button variant="ghost" size="icon" disabled aria-label="次の曲">
        <SkipForward class="size-4" />
      </Button>
      <Button variant="ghost" size="icon" disabled aria-label="リピート">
        <Repeat class="size-4" />
      </Button>
    </div>
  </section>
</template>
