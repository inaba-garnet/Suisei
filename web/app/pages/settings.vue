<script setup lang="ts">
import type { ThemePreference } from '~/utils/theme'
import { LogOut, Monitor, Moon, Sun } from 'lucide-vue-next'

useHead({ title: '設定' })

const { user, logout } = useAuth()
const { preference, setPreference } = useTheme()

const themes: { value: ThemePreference, label: string, icon: typeof Sun }[] = [
  { value: 'system', label: 'OS に合わせる', icon: Monitor },
  { value: 'light', label: 'ライト', icon: Sun },
  { value: 'dark', label: 'ダーク', icon: Moon },
]

const loggingOut = ref(false)

async function onLogout() {
  loggingOut.value = true
  try {
    await logout()
    await navigateTo('/login')
  }
  finally {
    loggingOut.value = false
  }
}
</script>

<template>
  <div>
    <PageHeader title="設定" />

    <section class="flex flex-col gap-3">
      <h2 class="text-h3 font-semibold">
        テーマ
      </h2>
      <!-- Design の SegmentedControl に合わせる -->
      <div role="radiogroup" aria-label="テーマ" class="inline-flex w-fit gap-0.5 rounded-md border border-border-subtle bg-surface-1 p-0.5">
        <button
          v-for="theme in themes"
          :key="theme.value"
          type="button"
          role="radio"
          :aria-checked="preference === theme.value"
          class="inline-flex h-8 items-center gap-1.5 rounded-sm border border-transparent px-3 text-body text-fg-muted transition-all hover:text-fg aria-checked:border-border-strong aria-checked:bg-accent-soft aria-checked:text-fg"
          @click="setPreference(theme.value)"
        >
          <component :is="theme.icon" class="size-3.5" />
          {{ theme.label }}
        </button>
      </div>
    </section>

    <section class="mt-8 flex flex-col gap-3">
      <h2 class="text-h3 font-semibold">
        アカウント
      </h2>
      <div class="flex items-center gap-4 rounded-lg border border-border-subtle bg-surface-1 p-4">
        <span class="flex size-9 items-center justify-center rounded-full border border-border-strong bg-accent-soft text-body font-medium">
          {{ user?.charAt(0).toUpperCase() }}
        </span>
        <span class="flex-1 truncate text-body-lg" data-testid="username">{{ user }}</span>
        <Button variant="secondary" :disabled="loggingOut" @click="onLogout">
          <LogOut class="size-3.5" />
          ログアウト
        </Button>
      </div>
    </section>
  </div>
</template>
