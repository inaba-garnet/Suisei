<script setup lang="ts">
import { ChevronRight } from 'lucide-vue-next'
import { libraryNav } from '~/utils/navigation'

useHead({ title: 'ライブラリ' })
</script>

<template>
  <div>
    <PageHeader title="ライブラリ" />
    <ul class="overflow-hidden rounded-lg border border-border-subtle bg-surface-1">
      <li v-for="item in libraryNav" :key="item.to" class="border-b border-divider last:border-b-0">
        <NuxtLink :to="item.to" class="flex h-14 items-center gap-3 px-4 transition-colors hover:bg-surface-2">
          <component :is="item.icon" class="size-5 text-accent-base" />
          <span class="flex-1 text-body-lg">{{ item.label }}</span>
          <ChevronRight class="size-4 text-fg-subtle" />
        </NuxtLink>
      </li>
    </ul>
  </div>
</template>
