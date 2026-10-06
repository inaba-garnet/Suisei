<script setup lang="ts">
import { useIntervalFn } from '@vueuse/core'
import { RefreshCw } from 'lucide-vue-next'
import { scanReportMessage } from '~/utils/scan'
import { backupIntervals, backupKeeps, intervalLabel, scanIntervals, withCurrent } from '~/utils/settings'

/** 設定の画面の「ライブラリ」と「バックアップ」（docs/web.md の「設定」）。選んだらすぐ保存する。 */
const { settings, saving, update } = useServerSettings()
const scan = useLibraryScan()
// 定期のスキャンが走っていれば、その進みを出す。走っていなければ最後の結果を出す
onMounted(scan.refresh)
// 「5 分前」の表示を進める
const now = ref(Date.now())
useIntervalFn(() => {
  now.value = Date.now()
}, 30_000)
watch(() => scan.last.value, () => {
  now.value = Date.now()
})

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
        <div class="flex items-center gap-3 border-b border-divider px-3 py-3">
          <p class="min-w-0 flex-1 text-body-sm text-fg-subtle" data-testid="scan-status">
            <template v-if="scan.scanning.value">
              スキャン中…（{{ scan.count.value }} 件）
            </template>
            <template v-else-if="scan.last.value">
              <span :class="{ 'text-danger': scan.last.value.error }">{{ scanReportMessage(scan.last.value, now) }}</span>
            </template>
            <template v-else>
              曲を足したりタグを直したりしたら、すぐに反映できます
            </template>
          </p>
          <Button variant="secondary" :disabled="scan.scanning.value" @click="scan.start()">
            <RefreshCw class="size-3.5" :class="{ 'animate-spin': scan.scanning.value }" />
            今すぐスキャン
          </Button>
        </div>
        <details v-if="!scan.scanning.value && scan.last.value?.failed" class="border-b border-divider px-3 py-3" data-testid="scan-failed">
          <summary class="cursor-pointer text-body-sm text-danger">
            読めなかったファイル {{ scan.last.value.failed }} 件
          </summary>
          <p class="mt-2 text-body-sm text-fg-subtle">
            壊れているか、対応していない形式のファイルです。前のスキャンで読めていれば、その内容のまま残しています。
          </p>
          <ul class="mt-2 flex flex-col gap-1 text-body-sm break-all">
            <li v-for="path in scan.last.value.failedPaths" :key="path">
              <code>{{ path }}</code>
            </li>
            <li v-if="(scan.last.value.failedPaths?.length ?? 0) < scan.last.value.failed" class="text-fg-subtle">
              ほか {{ scan.last.value.failed - (scan.last.value.failedPaths?.length ?? 0) }} 件
            </li>
          </ul>
        </details>
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
