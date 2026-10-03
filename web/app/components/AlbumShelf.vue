<script setup lang="ts">
import type { Album } from '~/utils/subsonic'

/**
 * アルバムのジャケットを横スクロールで一列に並べる（docs/web.md の「一覧」）。スマホでは画面の端まで広げる。
 * 名前の下には `caption` を出す。既定は年（アーティストの詳細ではアーティストが分かっているため）。
 */
withDefaults(defineProps<{ albums: Album[], caption?: 'year' | 'artist' }>(), { caption: 'year' })
</script>

<template>
  <ul class="-mx-4 flex snap-x gap-4 overflow-x-auto scroll-px-4 px-4 pb-2 md:mx-0 md:scroll-px-0 md:px-0" data-testid="album-shelf">
    <li v-for="album in albums" :key="album.id" class="w-36 shrink-0 snap-start">
      <NuxtLink :to="`/library/albums/${album.id}`" class="group flex min-w-0 flex-col gap-2">
        <CoverArt
          :id="album.coverArt"
          :size="150"
          :alt="album.name"
          class="transition-all group-hover:border-border-strong group-hover:shadow-[0_0_14px_var(--glow-low)]"
        />
        <span class="min-w-0">
          <span class="block truncate text-body font-medium">{{ album.name }}</span>
          <span class="block truncate text-body-sm text-fg-subtle">{{ (caption === 'artist' ? album.artist : album.year) ?? '' }}</span>
        </span>
      </NuxtLink>
    </li>
  </ul>
</template>
