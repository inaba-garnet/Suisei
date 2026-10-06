<script setup lang="ts">
import { backupIntervals, backupKeeps, intervalLabel, scanIntervals, withCurrent } from '~/utils/settings'

/** 設定の画面の「ライブラリ」と「バックアップ」（docs/web.md の「設定」）。選んだらすぐ保存する。 */
const { settings, saving, update } = useServerSettings()

const scanOptions = computed(() =>
  withCurrent(scanIntervals, settings.value?.scanInterval ?? 0).map(value => ({ value, label: intervalLabel(value) })))
const backupOptions = computed(() =>
  withCurrent(backupIntervals, settings.value?.backupInterval ?? 0).map(value => ({ value, label: intervalLabel(value) })))
const keepOptions = computed(() =>
  withCurrent(backupKeeps, settings.value?.backupKeep ?? 1).map(value => ({ value, label: `${value} 世代` })))

const scanInterval = computed({
  get: () => settings.value?.scanInterval ?? 0,
  set: value => update({ scanInterval: value }),
})
const backupInterval = computed({
  get: () => settings.value?.backupInterval ?? 0,
  set: value => update({ backupInterval: value }),
})
const backupKeep = computed({
  get: () => settings.value?.backupKeep ?? 1,
  set: value => update({ backupKeep: value }),
})
</script>

<template>
  <template v-if="settings">
    <section class="mt-8 flex flex-col gap-3">
      <h2 class="text-h3 font-semibold">
        ライブラリ
      </h2>
      <div class="border-t border-divider">
        <SettingSelect
          v-model="scanInterval"
          label="スキャンの間隔"
          description="音楽フォルダを読み直す間隔。前のスキャンが終わってから数えます"
          :options="scanOptions"
          :disabled="saving"
        />
      </div>
    </section>

    <section class="mt-8 flex flex-col gap-3">
      <h2 class="text-h3 font-semibold">
        バックアップ
      </h2>
      <div class="border-t border-divider">
        <SettingSelect
          v-model="backupInterval"
          label="データベースのバックアップの間隔"
          :options="backupOptions"
          :disabled="saving"
        />
        <SettingSelect
          v-model="backupKeep"
          label="残す数"
          :options="keepOptions"
          :disabled="saving || backupInterval === 0"
        />
      </div>
    </section>
  </template>
</template>
