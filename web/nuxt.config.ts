import tailwindcss from '@tailwindcss/vite'
import { thirdPartyLicenses } from './build/third-party-licenses'

// 開発サーバーから `/api` と `/rest` を中継する先（docs/web.md）
const backend = process.env.SUISEI_DEV_BACKEND ?? 'http://127.0.0.1:4533'

export default defineNuxtConfig({
  compatibilityDate: '2026-09-01',
  // 静的に書き出してサーバーのバイナリに埋め込むので、SSR はしない
  ssr: false,
  devtools: { enabled: false },
  modules: ['@nuxt/eslint', 'shadcn-nuxt'],
  css: ['~/assets/css/main.css'],
  app: {
    head: {
      htmlAttrs: { lang: 'ja' },
      title: 'Suisei',
      meta: [
        // iPhone の画面の端（ホームバーなど）を避けて配置するため
        { name: 'viewport', content: 'width=device-width, initial-scale=1, viewport-fit=cover' },
        { name: 'theme-color', content: '#0B1020' },
      ],
      // アプリのアイコン（docs/web.md の「デザイン」）
      link: [
        { rel: 'icon', type: 'image/png', sizes: '32x32', href: '/favicon-32.png' },
        { rel: 'icon', type: 'image/png', sizes: '192x192', href: '/icon-192.png' },
        { rel: 'apple-touch-icon', href: '/apple-touch-icon.png' },
      ],
      script: [
        {
          // 描画の前にテーマを決め、ライトのときに暗い画面が一瞬出ないようにする（utils/theme.ts と同じ判定）
          innerHTML: `try{var t=localStorage.getItem('suisei-theme');if(t!=='light'&&t!=='dark')t=matchMedia('(prefers-color-scheme: light)').matches?'light':'dark';document.documentElement.dataset.theme=t}catch(e){}`,
        },
      ],
    },
  },
  shadcn: {
    prefix: '',
    componentDir: './app/components/ui',
  },
  vite: {
    plugins: [tailwindcss()],
  },
  hooks: {
    // 配るのはクライアントのビルド結果だけなので、その依存のライセンスを書き出す
    'vite:extendConfig'(config, { isClient }) {
      if (isClient) {
        config.plugins?.push(thirdPartyLicenses())
      }
    },
  },
  nitro: {
    devProxy: {
      '/api': { target: `${backend}/api`, changeOrigin: true },
      '/rest': { target: `${backend}/rest`, changeOrigin: true },
    },
  },
})
