<script setup lang="ts">
import type { Component } from 'vue'

defineProps<{ icon: Component, title: string, description?: string }>()
</script>

<template>
  <div class="flex flex-col items-center gap-3 rounded-lg border border-dashed border-border-default px-6 py-12 text-center">
    <component :is="icon" class="size-8 text-fg-subtle" />
    <p class="text-h3 font-semibold">
      {{ title }}
    </p>
    <p v-if="description" class="max-w-sm text-body text-fg-subtle">
      {{ description }}
    </p>
  </div>
</template>
