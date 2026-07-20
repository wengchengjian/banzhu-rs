import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { VitePWA } from 'vite-plugin-pwa'
import { fileURLToPath, URL } from 'node:url'

export default defineConfig({
  plugins: [
    vue(),
    VitePWA({
      registerType: 'autoUpdate',
      manifest: {
        name: '版主网阅读',
        short_name: '版主网',
        theme_color: '#3b82f6',
        background_color: '#ffffff',
        display: 'standalone',
        icons: [
          { src: '/icons/192.png', sizes: '192x192', type: 'image/png' },
          { src: '/icons/512.png', sizes: '512x512', type: 'image/png' },
          { src: '/icons/maskable.png', sizes: '512x512', type: 'image/png', purpose: 'maskable' },
        ],
      },
      workbox: {
        runtimeCaching: [
          // 章节内容：NetworkFirst，缓存到 chapters-cache
          {
            urlPattern: /\/api\/books\/\d+\/chapters\/\d+$/,
            handler: 'NetworkFirst',
            options: {
              cacheName: 'chapters-cache',
              expiration: { maxEntries: 5000 }, // 永久缓存，限制上限防止失控
              networkTimeoutSeconds: 10, // 修复 Task 22 I3：弱网 10s 超时回退缓存
            },
          },
          // 书籍详情：NetworkFirst，容错
          {
            urlPattern: /\/api\/books\/\d+$/,
            handler: 'NetworkFirst',
            options: {
              cacheName: 'books-cache',
              expiration: { maxEntries: 200 },
            },
          },
          // 爬虫/统计：NetworkOnly（不缓存）
          // 其他 API：默认不缓存
        ],
      },
    }),
  ],
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
  server: {
    port: 5173,
    proxy: { '/api': { target: 'http://127.0.0.1:3000', changeOrigin: true } },
  },
  build: { outDir: 'dist', emptyOutDir: true },
})
