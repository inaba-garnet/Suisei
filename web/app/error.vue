<script setup lang="ts">
import type { NuxtError } from '#app'
import { CircleAlert } from 'lucide-vue-next'

defineProps<{ error: NuxtError }>()
</script>

<template>
  <main class="flex min-h-dvh items-center justify-center px-4">
    <div class="flex max-w-sm flex-col items-center gap-4 text-center">
      <CircleAlert class="size-8 text-danger" />
      <p class="text-h3 font-semibold">
        {{ error.statusCode === 404 ? 'ページが見つかりません' : 'サーバーに接続できません' }}
      </p>
      <Button variant="secondary" @click="clearError({ redirect: '/' })">
        ホームに戻る
      </Button>
    </div>
  </main>
</template>
