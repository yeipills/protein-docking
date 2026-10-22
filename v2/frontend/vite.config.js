import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [
    sveltekit({
      // Configuración actualizada para vite-plugin-svelte@4
      inspector: {
        holdMode: true,
        showToggleButton: 'always',
        toggleButtonPos: 'bottom-right'
      }
    })
  ],
  
  // Optimizaciones de performance máximo
  build: {
    target: 'es2022',
    minify: 'esbuild',
    sourcemap: false,
    rollupOptions: {
      output: {
        // Chunks automáticos sin dependencias específicas
        manualChunks: (id) => {
          if (id.includes('node_modules')) {
            return 'vendor';
          }
        }
      }
    }
  },
  
  // Configuración de desarrollo optimizada
  server: {
    port: 3000,
    host: true,
    cors: true,
    hmr: {
      port: 3001
    }
  },
  
  // Nota: resolve.alias removido - SvelteKit maneja los aliases automáticamente
  // Los aliases se configuran en svelte.config.js usando kit.alias
  
  // Configuración de workers (actualizada para Vite 5+)
  worker: {
    format: 'es',
    plugins: () => []
  },
  
  // Optimizaciones de dependencias
  optimizeDeps: {
    include: []
  },
  
  // Configuración de testing
  test: {
    include: ['src/**/*.{test,spec}.{js,ts}'],
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    coverage: {
      reporter: ['text', 'html', 'lcov'],
      exclude: ['src/test/**']
    }
  },
  
  // Variables de entorno
  define: {
    __BUILD_DATE__: JSON.stringify(new Date().toISOString()),
    __VERSION__: JSON.stringify(process.env.npm_package_version || '2.0.0')
  }
});