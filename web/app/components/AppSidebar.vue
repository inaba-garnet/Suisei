<script setup lang="ts">
import { isUnder, libraryNav, mainNav } from '~/utils/navigation'

const route = useRoute()

/** ライブラリの下の画面では、展開した項目のほうを選択中にする。 */
function isActive(to: string) {
  return to === '/library' ? route.path === '/library' : isUnder(route.path, to)
}
</script>

<template>
  <!-- PC のサイドバー。ライブラリの項目は最初から展開して出す（docs/web.md） -->
  <nav aria-label="メイン" class="flex h-full flex-col gap-0.5 overflow-y-auto px-3 pt-4 pb-3">
    <NuxtLink to="/" class="mb-4 px-2">
      <AppLogo />
    </NuxtLink>
    <template v-for="item in mainNav" :key="item.to">
      <NuxtLink
        :to="item.to"
        class="nav-item"
        :data-active="isActive(item.to) ? '' : undefined"
      >
        <component :is="item.icon" class="nav-icon size-4" />
        {{ item.label }}
      </NuxtLink>
      <div v-if="item.to === '/library'" class="mb-2 flex flex-col gap-0.5 pl-4">
        <NuxtLink
          v-for="sub in libraryNav"
          :key="sub.to"
          :to="sub.to"
          class="nav-item"
          :data-active="isUnder(route.path, sub.to) ? '' : undefined"
        >
          <component :is="sub.icon" class="nav-icon size-4" />
          {{ sub.label }}
        </NuxtLink>
      </div>
    </template>
  </nav>
</template>

<style scoped>
@reference '~/assets/css/main.css';

/* Design の SidebarNav に合わせる */
.nav-item {
  @apply flex items-center gap-2.5 rounded-md border border-transparent px-3 py-[7px] text-body text-fg-muted transition-all hover:bg-surface-2 hover:text-fg;
}

.nav-item[data-active] {
  @apply border-border-strong bg-accent-soft text-fg shadow-glow-soft;
}

.nav-icon {
  @apply shrink-0 text-fg-subtle;
}

.nav-item[data-active] .nav-icon {
  @apply text-accent-active;
}
</style>
