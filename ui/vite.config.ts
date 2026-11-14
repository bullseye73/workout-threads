import { defineConfig } from 'vitest/config'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'
import { fileURLToPath } from 'node:url'

// 폰트는 카드 렌더러와 같은 파일(crates/card/assets/fonts)을 그대로 쓴다.
export default defineConfig({
  plugins: [tailwindcss(), svelte()],
  resolve: {
    alias: { $lib: fileURLToPath(new URL('./src/lib', import.meta.url)) },
    // 컴포넌트 테스트(happy-dom)에서 svelte의 브라우저 빌드를 쓰게 한다.
    conditions: process.env.VITEST ? ['browser'] : undefined,
  },
  clearScreen: false,
  server: { port: 5173, strictPort: true, fs: { allow: ['..'] } },
  // 시간 표시 테스트가 어느 PC·CI에서든 같게.
  test: { env: { TZ: 'Asia/Seoul' } },
})
