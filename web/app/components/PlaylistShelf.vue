<script setup lang="ts">
import type { RouteLocationRaw } from 'vue-router'
import type { Playlist } from '~/utils/subsonic'

/** 見出しとプレイリストのジャケットの横スクロールの棚（docs/web.md の「ホーム」）。アルバムの棚と同じ形にする。 */
defineProps<{ title: string, playlists: Playlist[], more?: RouteLocationRaw }>()
</script>

<template>
  <ShelfSection :title="title" :count="playlists.length" :more="more" testid="playlist-shelf">
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
    <template #fallback>
      <slot name="fallback" />
    </template>
  </ShelfSection>
</template>
