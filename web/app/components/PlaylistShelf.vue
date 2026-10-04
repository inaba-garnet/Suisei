<script setup lang="ts">
import type { Playlist } from '~/utils/subsonic'

/** プレイリストのジャケットを横スクロールで一列に並べる（docs/web.md の「ホーム」）。アルバムの棚と同じ形にする。 */
defineProps<{ playlists: Playlist[] }>()
</script>

<template>
  <ul class="-mx-4 flex snap-x gap-4 overflow-x-auto scroll-px-4 px-4 pb-2 md:mx-0 md:scroll-px-0 md:px-0" data-testid="playlist-shelf">
    <li v-for="playlist in playlists" :key="playlist.id" class="w-36 shrink-0 snap-start">
      <NuxtLink :to="`/playlists/${playlist.id}`" class="group flex min-w-0 flex-col gap-2">
        <CoverArt
          :id="playlist.coverArt"
          :size="150"
          :alt="playlist.name"
          class="transition-all group-hover:border-border-strong group-hover:shadow-[0_0_14px_var(--glow-low)]"
        />
        <span class="min-w-0">
          <span class="block truncate text-body font-medium">{{ playlist.name }}</span>
          <span class="block truncate text-body-sm text-fg-subtle">{{ playlist.songCount }} 曲</span>
        </span>
      </NuxtLink>
    </li>
  </ul>
</template>
