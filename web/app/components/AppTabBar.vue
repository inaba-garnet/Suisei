<script setup lang="ts">
import { isUnder, mainNav } from '~/utils/navigation'

const route = useRoute()
</script>

<template>
  <!-- スマホの下のタブ。表示中のタブを押すと、そのタブの最初の画面に移る（docs/web.md） -->
  <nav aria-label="メイン" class="grid grid-cols-5 border-t border-divider bg-surface-1/95 pb-[env(safe-area-inset-bottom)] backdrop-blur">
    <NuxtLink
      v-for="item in mainNav"
      :key="item.to"
      :to="item.to"
      class="tab"
      :data-active="isUnder(route.path, item.to) ? '' : undefined"
      :aria-current="isUnder(route.path, item.to) ? 'page' : undefined"
    >
      <component :is="item.icon" class="size-5" />
      <span class="text-caption">{{ item.label }}</span>
    </NuxtLink>
  </nav>
</template>

<style scoped>
@reference '~/assets/css/main.css';

.tab {
  @apply flex h-14 flex-col items-center justify-center gap-1 text-fg-subtle transition-colors;
}

.tab[data-active] {
  @apply text-accent-base;
}
</style>
