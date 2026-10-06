<script setup lang="ts">
/**
 * 設定の画面の「詳細」（docs/web.md の「設定」）。`キャラクター(CV:声優)` の分け方は、
 * 使い始めてから変えるとお気に入りや評価が別のアーティストに移ることがあるので、閉じておく。
 */
const { settings, saving, update } = useServerSettings()

const split = computed({
  get: () => (settings.value?.splitCharacters ?? true) ? 'split' : 'keep',
  set: (value: 'split' | 'keep') => update({ splitCharacters: value === 'split' }),
})
const splitOptions = [
  { value: 'split' as const, label: '分ける' },
  { value: 'keep' as const, label: '分けない' },
]
</script>

<template>
  <section v-if="settings" class="mt-8">
    <details class="group">
      <summary class="cursor-pointer text-h3 font-semibold">
        詳細
      </summary>
      <div class="mt-3 flex flex-col gap-3">
        <p class="text-body">
          <code>キャラクター(CV:声優)</code> の形のアーティスト名を分ける
        </p>
        <p class="text-body-sm text-fg-subtle">
          使い始めてから変えると、次のスキャンでアーティストが入れ替わり、お気に入りや評価が別のアーティストに移ることがあります。
        </p>
        <SegmentedControl v-model="split" label="キャラクターと声優の分け方" :options="splitOptions.map(o => ({ ...o, disabled: saving }))" />
      </div>
    </details>
  </section>
</template>
