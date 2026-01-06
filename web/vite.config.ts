import { defineConfig } from 'vite'
import solid from 'vite-plugin-solid'

export default defineConfig(({ mode }) => ({
  plugins: [solid()],
  server: {
    port: 5173,
    proxy: {
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true,
        ws: true,  // WebSocketプロキシを有効化
      },
    },
  },
  build: {
    outDir: 'dist',
    // Source maps only in development (disable in production for security/size)
    sourcemap: mode === 'development',
    // Optimize chunk splitting
    rollupOptions: {
      output: {
        // Manual chunk splitting for better caching
        manualChunks: {
          // Vendor chunk for SolidJS core
          'solid-vendor': ['solid-js', 'solid-js/web', 'solid-js/store'],
          // Router chunk
          'router': ['@solidjs/router'],
        },
      },
    },
    // Increase chunk size warning limit (default is 500kb)
    chunkSizeWarningLimit: 600,
  },
}))
