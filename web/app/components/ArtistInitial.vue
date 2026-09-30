<script setup lang="ts">
/** アーティストの画像の代わりに、名前の頭文字を丸に入れて出す（docs/web.md の「一覧」）。 */
const props = defineProps<{ name: string }>()
const initial = computed(() => Array.from(props.name.trim())[0]?.toUpperCase() ?? '')
</script>

<template>
  <span aria-hidden="true" class="flex shrink-0 items-center justify-center rounded-full border border-border-subtle bg-surface-3 font-medium text-fg-muted">
    {{ initial }}
  </span>
</template>
