// پیکربندی Vite.
//
// چرا لازم است: فایل‌های JS از بسته‌هایی مثل `@tauri-apps/api/core` import می‌کنند.
// مرورگر خودش این نام‌ها را نمی‌شناسد؛ باید قبل از بیلد یکجا شوند.
// بدون این مرحله، پنجرهٔ برنامه سفید بالا می‌آمد.
import { defineConfig } from 'vite'

export default defineConfig({
  root: 'src',
  build: {
    outDir: '../dist-web',
    emptyOutDir: true,
    target: 'chrome110', // WebView2 روی ویندوز ۱۰
    sourcemap: false,
  },
  clearScreen: false,
  server: { port: 5173, strictPort: true },
})
