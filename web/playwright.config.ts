import { defineConfig, devices } from '@playwright/test'

const port = 3100

// 書き出した SPA を配り、/api と /rest の応答はテストの中で差し替える（docs/web.md）
export default defineConfig({
  testDir: 'test/e2e',
  forbidOnly: !!process.env.CI,
  reporter: process.env.CI ? 'github' : 'list',
  use: {
    baseURL: `http://127.0.0.1:${port}`,
    locale: 'ja-JP',
    launchOptions: {
      // 手元のブラウザが Playwright の版と合わないときに差し替える
      executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE || undefined,
    },
  },
  projects: [
    { name: 'desktop', use: { ...devices['Desktop Chrome'], viewport: { width: 1280, height: 800 } } },
    { name: 'mobile', use: { ...devices['Pixel 7'] } },
  ],
  webServer: {
    command: 'node test/e2e/serve.mjs',
    url: `http://127.0.0.1:${port}/`,
    env: { PORT: String(port) },
    reuseExistingServer: !process.env.CI,
  },
})
