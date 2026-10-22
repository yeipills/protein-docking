import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// In development the API runs on `cshape serve` (port 8080).
export default defineConfig({
  plugins: [react()],
  server: {
    proxy: { '/api': 'http://localhost:8080' },
  },
  build: {
    chunkSizeWarningLimit: 1500,
  },
})
