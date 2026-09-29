<script setup lang="ts">
import { CircleAlert, LoaderCircle } from 'lucide-vue-next'
import { safeRedirect } from '~/utils/redirect'

definePageMeta({ layout: false })
useHead({ title: 'ログイン' })

const { login } = useAuth()
const route = useRoute()

const username = ref('')
const password = ref('')
const error = ref('')
const pending = ref(false)

async function onSubmit() {
  error.value = ''
  pending.value = true
  try {
    await login(username.value, password.value)
    await navigateTo(safeRedirect(route.query.redirect), { replace: true })
  }
  catch (err) {
    error.value = err instanceof LoginError ? err.message : 'サーバーに接続できません'
  }
  finally {
    pending.value = false
  }
}
</script>

<template>
  <main class="flex min-h-dvh items-center justify-center px-4 py-10">
    <div class="w-full max-w-sm">
      <div class="mb-8 flex justify-center">
        <AppLogo :size="36" />
      </div>
      <form
        class="flex flex-col gap-5 rounded-lg border border-border-default bg-surface-3 p-5 shadow-elevation-4"
        @submit.prevent="onSubmit"
      >
        <div class="flex flex-col gap-2">
          <Label for="username">ユーザー名</Label>
          <Input id="username" v-model="username" name="username" autocomplete="username" autocapitalize="none" required />
        </div>
        <div class="flex flex-col gap-2">
          <Label for="password">パスワード</Label>
          <Input id="password" v-model="password" name="password" type="password" autocomplete="current-password" required />
        </div>
        <p v-if="error" role="alert" class="flex items-center gap-2 rounded-md border border-danger/35 bg-danger-bg px-3 py-2 text-body text-danger">
          <CircleAlert class="size-4 shrink-0" />
          {{ error }}
        </p>
        <Button type="submit" size="lg" :disabled="pending">
          <LoaderCircle v-if="pending" class="size-4 animate-spin" />
          ログイン
        </Button>
      </form>
    </div>
  </main>
</template>
