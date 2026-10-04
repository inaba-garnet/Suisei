<script setup lang="ts">
import type { RouteLocationRaw } from 'vue-router'
import type { Album } from '~/utils/subsonic'

/**
 * 見出しとアルバムのジャケットの横スクロールの棚（docs/web.md の「一覧」）。スマホでは画面の端まで広げる。
 * 名前の下には `caption` を出す。既定は年（アーティストの詳細ではアーティストが分かっているため）。
 */
withDefaults(defineProps<{ title: string, albums: Album[], caption?: 'year' | 'artist', more?: RouteLocationRaw }>(), { caption: 'year', more: undefined })
</script>

<template>
  <ShelfSection :title="title" :count="albums.length" :more="more" testid="album-shelf">
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
    <template #fallback>
      <slot name="fallback" />
    </template>
  </ShelfSection>
</template>
