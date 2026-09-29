<script setup lang="ts">
import { Music, Play, SkipBack, SkipForward } from 'lucide-vue-next'
</script>

<template>
  <!--
    再生バーの枠。スマホではタブの上の小さな帯、PC では下端いっぱいに出す。
    再生を入れるまでは、何も再生していない状態だけを出す。
  -->
  <div
    class="mx-2 mb-2 flex items-center gap-3 rounded-lg border border-border-subtle bg-surface-2 p-2 shadow-elevation-2 md:m-0 md:h-full md:rounded-none md:border-0 md:bg-transparent md:px-4 md:py-0 md:shadow-none"
    data-testid="player-bar"
  >
    <div class="flex size-10 shrink-0 items-center justify-center rounded-sm border border-border-subtle bg-surface-3 text-fg-subtle md:size-12">
      <Music class="size-5" />
    </div>
    <div class="min-w-0 flex-1">
      <p class="truncate text-body text-fg-muted">
        再生していません
      </p>
    </div>
    <div class="flex items-center gap-1">
      <Button variant="ghost" size="icon" class="hidden md:inline-flex" disabled aria-label="前の曲">
        <SkipBack class="size-4" />
      </Button>
      <Button variant="default" size="icon" class="rounded-full" disabled aria-label="再生">
        <Play class="size-4" />
      </Button>
      <Button variant="ghost" size="icon" class="hidden md:inline-flex" disabled aria-label="次の曲">
        <SkipForward class="size-4" />
      </Button>
    </div>
  </div>
</template>
