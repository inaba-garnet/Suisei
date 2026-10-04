<script setup lang="ts">
/** 画面の下に短く出す知らせ（docs/web.md の「メニュー」）。再生バーの上に重ねる。 */
const { message } = useToast()
</script>

<template>
  <div class="pointer-events-none fixed inset-x-0 bottom-[calc(env(safe-area-inset-bottom)+136px)] z-50 flex justify-center px-4 md:bottom-24" aria-live="polite">
    <Transition
      mode="out-in"
      enter-from-class="translate-y-2 opacity-0"
      leave-to-class="translate-y-2 opacity-0"
      enter-active-class="transition duration-200"
      leave-active-class="transition duration-200"
    >
      <p
        v-if="message"
        :key="message.id"
        role="status"
        class="max-w-sm truncate rounded-full border border-border-default bg-surface-3 px-4 py-2 text-body text-fg shadow-elevation-3"
      >
        {{ message.text }}
      </p>
    </Transition>
  </div>
</template>
