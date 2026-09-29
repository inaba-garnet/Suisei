<script setup lang="ts">
defineProps<{ title: string, description?: string }>()
</script>

<template>
  <header class="mb-5 flex items-baseline gap-2.5">
    <h1 class="text-h1 font-semibold">
      {{ title }}
    </h1>
    <span v-if="description" class="text-body-sm text-fg-subtle">{{ description }}</span>
  </header>
</template>
