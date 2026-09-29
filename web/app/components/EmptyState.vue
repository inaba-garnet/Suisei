<script setup lang="ts">
import type { Component } from 'vue'

defineProps<{ icon: Component, title: string, description?: string }>()
</script>

<template>
  <div class="flex flex-col items-center gap-1.5 px-6 py-8 text-center">
    <component :is="icon" class="mb-2 size-8 text-fg-subtle opacity-70" />
    <p class="text-h3 font-semibold">
      {{ title }}
    </p>
    <p v-if="description" class="max-w-xs text-body-sm text-fg-subtle">
      {{ description }}
    </p>
  </div>
</template>
