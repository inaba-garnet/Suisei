<script setup lang="ts">
import type { Component } from 'vue'
import type { ContentAlign } from '~/utils/layout'
import type { ThemePreference } from '~/utils/theme'
import { AlignCenter, AlignLeft, LogOut, Monitor, Moon, Sun } from 'lucide-vue-next'

useHead({ title: '設定' })

const { user, logout } = useAuth()
const { preference, setPreference } = useTheme()
const { align, setAlign } = useContentAlign()

const themeModel = computed({ get: () => preference.value, set: setPreference })
const alignModel = computed({ get: () => align.value, set: setAlign })

const themes: { value: ThemePreference, label: string, icon: Component }[] = [
  { value: 'system', label: 'OS に合わせる', icon: Monitor },
  { value: 'light', label: 'ライト', icon: Sun },
  { value: 'dark', label: 'ダーク', icon: Moon },
]

const aligns: { value: ContentAlign, label: string, icon: Component }[] = [
  { value: 'center', label: '中央', icon: AlignCenter },
  { value: 'left', label: '左寄せ', icon: AlignLeft },
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
      <SegmentedControl v-model="themeModel" label="テーマ" :options="themes" />
    </section>

    <section class="mt-8 flex flex-col gap-3">
      <h2 class="text-h3 font-semibold">
        開発
      </h2>
      <p class="text-body text-fg-subtle">
        PC で内容を置く位置を見比べるための一時的な項目です。
      </p>
      <SegmentedControl v-model="alignModel" label="内容の配置" :options="aligns" />
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
