import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  // Consult https://kit.svelte.dev/docs/integrations#preprocessors
  preprocess: vitePreprocess(),

  kit: {
    adapter: adapter({
      // Output to build directory for nginx
      pages: 'build',
      assets: 'build',
      fallback: 'index.html',
      precompress: true,
      strict: false
    }),
    
    // Alias para imports
    alias: {
      '$components': 'src/lib/components',
      '$stores': 'src/lib/stores',
      '$utils': 'src/lib/utils',
      '$types': 'src/lib/types',
      '$workers': 'src/lib/workers',
      '$wasm': 'static/wasm'
    },
    
    // Optimizaciones para máximo rendimiento
    prerender: {
      handleHttpError: 'warn',
      handleMissingId: 'warn'
    },
    
    // Service worker deshabilitado temporalmente
    serviceWorker: {
      register: false
    },
    
    // Configuración de rutas
    files: {
      assets: 'static',
      hooks: {
        client: 'src/hooks.client.ts',
        server: 'src/hooks.server.ts'
      },
      lib: 'src/lib',
      params: 'src/params',
      routes: 'src/routes',
      serviceWorker: 'src/service-worker.ts',
      appTemplate: 'src/app.html'
    },
    
    // Configuración de TypeScript
    typescript: {
      config: (config) => {
        return {
          ...config,
          compilerOptions: {
            ...config.compilerOptions,
            // Optimizaciones de compilación
            target: 'es2022',
            moduleResolution: 'bundler',
            allowImportingTsExtensions: false,
            strict: true,
            noImplicitReturns: true,
            noFallthroughCasesInSwitch: true,
            noImplicitOverride: true,
            exactOptionalPropertyTypes: true
          }
        };
      }
    },
    
    // CSP con todas las conexiones permitidas para desarrollo
    csp: {
      mode: 'auto',
      directives: {}
    }
  },
  
  // Configuración del compilador Svelte para máximo rendimiento
  compilerOptions: {
    hydratable: false, // SSG only, no hydration
    immutable: true,   // Optimización máxima
    legacy: false,     // Solo navegadores modernos
    css: 'injected',   // CSS inline para mejor performance inicial
    enableSourcemap: false // Sin sourcemaps en producción
  },
  
  // Configuración de vite-plugin-svelte@4 actualizada
  vitePlugin: {
    inspector: {
      holdMode: true,
      showToggleButton: 'active',
      toggleButtonPos: 'bottom-right'
    },
    // Optimizaciones de compilación
    compilerOptions: {
      dev: process.env.NODE_ENV === 'development',
      // HMR ahora se configura aquí en lugar de hot: true
      hmr: process.env.NODE_ENV === 'development'
    }
    
    // Nota: hot se removió - HMR ahora se configura en compilerOptions.hmr
    // experimental.inspector se movió a inspector (ya configurado arriba)
    // experimental.useVitePreprocess ya no es necesario en vite-plugin-svelte@4
  }
};

export default config;