<script setup lang="ts">
withDefaults(defineProps<{ size?: number }>(), { size: 28 })
</script>

<template>
  <!-- 四芒星のロゴ（docs/web.md）。色はアクセントに従うので、ライトでもそのまま使える -->
  <span class="inline-flex items-center gap-2.5">
    <svg
      :width="size"
      :height="size"
      viewBox="0 0 24 24"
      aria-hidden="true"
      class="text-accent-base drop-shadow-[0_0_6px_var(--glow-high)]"
    >
      <path
        d="M12 1.5c.7 5.2 4.3 8.8 9.5 9.5v1c-5.2.7-8.8 4.3-9.5 9.5h-1c-.7-5.2-4.3-8.8-9.5-9.5v-1C6.7 10.3 10.3 6.7 11 1.5h1Z"
        transform="translate(.5 .5) scale(.96)"
        fill="currentColor"
      />
    </svg>
    <span class="text-h2 font-semibold">Suisei</span>
  </span>
</template>
