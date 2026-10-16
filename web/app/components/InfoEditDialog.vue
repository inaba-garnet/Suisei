<script setup lang="ts">
import type { ReadingEntry } from '~/utils/reading'
import { DialogClose, DialogContent, DialogDescription, DialogOverlay, DialogPortal, DialogRoot, DialogTitle } from 'reka-ui'
import { sourceLabel } from '~/utils/reading'

/**
 * 曲、アルバム、アーティストの情報を直すモーダル（docs/web.md の「情報の編集」）。layout に一つだけ置く。
 * いまは読みだけを直す。開いたら今の読みと出どころを読み、手動の読みを保存するか、自動の読みに戻す。
 */
const { target, close } = useInfoEditor()
const { show } = useToast()

const entry = ref<ReadingEntry>()
const loadFailed = ref(false)
const input = ref('')
const busy = ref(false)
const error = ref<string>()

const open = computed({
  get: () => !!target.value,
  set: (value) => {
    if (!value) {
      close()
    }
  },
})

watch(target, async (t) => {
  entry.value = undefined
  loadFailed.value = false
  error.value = undefined
  input.value = ''
  if (!t) {
    return
  }
  try {
    const loaded = await $fetch<ReadingEntry>(`/api/readings/${encodeURIComponent(t.id)}`)
    // 読み込む間に別の対象を開き直していたら捨てる
    if (target.value?.id === t.id) {
      entry.value = loaded
      input.value = loaded.reading ?? ''
    }
  }
  catch {
    loadFailed.value = true
  }
})

/** `reading` が null なら手動の読みを外す。 */
async function save(reading: string | null) {
  const t = target.value
  if (!t || busy.value) {
    return
  }
  busy.value = true
  error.value = undefined
  try {
    await $fetch(`/api/readings/${encodeURIComponent(t.id)}`, { method: 'PUT', body: { reading } })
    close()
    show(reading === null ? '自動の読みに戻しました' : '保存しました')
    // 読みの順に並べた一覧を、直した読みで並べ直す
    await refreshNuxtData()
  }
  catch (err) {
    const data = (err as { data?: { error?: string } }).data
    error.value = data?.error === 'invalidReading'
      ? '読みはかなと英数字で入力してください（漢字は使えません）'
      : '保存できませんでした'
  }
  finally {
    busy.value = false
  }
}

function submit() {
  const reading = input.value.trim()
  if (!reading) {
    error.value = '読みを入力してください'
    return
  }
  save(reading)
}
</script>

<template>
  <DialogRoot v-model:open="open">
    <DialogPortal>
      <DialogOverlay class="fixed inset-0 z-50 bg-black/50" />
      <DialogContent
        class="fixed top-1/2 left-1/2 z-50 flex w-[calc(100vw-32px)] max-w-sm -translate-x-1/2 -translate-y-1/2 flex-col gap-4 rounded-xl border border-border-default bg-surface-2 p-5 shadow-elevation-3"
        data-testid="info-edit-dialog"
      >
        <div class="flex flex-col gap-1">
          <DialogTitle class="text-h3 font-semibold">
            情報を編集
          </DialogTitle>
          <DialogDescription class="break-words text-body text-fg-muted">
            {{ target?.name }}
          </DialogDescription>
        </div>

        <p v-if="loadFailed" class="text-body-sm text-danger">
          情報を読み込めませんでした
        </p>
        <p v-else-if="!entry" class="text-body-sm text-fg-subtle">
          読み込み中…
        </p>
        <form v-else class="flex flex-col gap-4" @submit.prevent="submit">
          <dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-body-sm">
            <dt class="text-fg-subtle">
              今の読み
            </dt>
            <dd data-testid="reading-current">
              {{ entry.reading ?? '—' }}（{{ sourceLabel(entry.source) }}）
            </dd>
            <template v-if="entry.source === 'manual'">
              <dt class="text-fg-subtle">
                自動の読み
              </dt>
              <dd data-testid="reading-auto">
                {{ entry.autoReading ?? '—' }}（{{ sourceLabel(entry.autoSource) }}）
              </dd>
            </template>
          </dl>
          <label class="flex flex-col gap-1.5">
            <span class="text-body-sm text-fg-muted">新しい読み</span>
            <Input v-model="input" autocomplete="off" enterkeyhint="done" placeholder="ひらがなかカタカナ" />
          </label>
          <p v-if="error" class="text-body-sm text-danger" role="alert">
            {{ error }}
          </p>
          <div class="flex flex-wrap items-center justify-end gap-2">
            <Button
              v-if="entry.source === 'manual'"
              type="button"
              variant="ghost"
              class="mr-auto"
              :disabled="busy"
              @click="save(null)"
            >
              自動の読みに戻す
            </Button>
            <DialogClose as-child>
              <Button type="button" variant="secondary">
                キャンセル
              </Button>
            </DialogClose>
            <Button type="submit" :disabled="busy">
              保存
            </Button>
          </div>
        </form>
      </DialogContent>
    </DialogPortal>
  </DialogRoot>
</template>
