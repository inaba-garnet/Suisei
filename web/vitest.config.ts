import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vitest/config'

// Nuxt に依存しない関数（app/utils）の単体テスト。画面は Playwright で確かめる
export default defineConfig({
  resolve: {
    alias: {
      '~': fileURLToPath(new URL('./app', import.meta.url)),
    },
  },
  test: {
    include: ['test/unit/**/*.test.ts'],
  },
})
