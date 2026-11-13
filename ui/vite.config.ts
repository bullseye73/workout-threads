import { defineConfig } from 'vitest/config'
import { svelte } from '@sveltejs/vite-plugin-svelte'

// 폰트는 카드 렌더러와 같은 파일(crates/card/assets/fonts)을 그대로 쓴다.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 5173, strictPort: true, fs: { allow: ['..'] } },
  // 시간 표시 테스트가 어느 PC·CI에서든 같게.
  test: { env: { TZ: 'Asia/Seoul' } },
})
