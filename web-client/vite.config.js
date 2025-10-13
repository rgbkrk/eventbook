import { defineConfig } from 'vite'

export default defineConfig({
  server: {
    port: 5173,
    proxy: {
      // Proxy API calls to our Rust server
      '/api': {
        target: 'http://localhost:3000',
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/api/, '')
      }
    }
  },

  build: {
    target: 'esnext',
    rollupOptions: {
      external: [],
    }
  },

  optimizeDeps: {
    exclude: ['eventbook-wasm']
  },

  // Enable WASM support
  assetsInclude: ['**/*.wasm'],

  define: {
    global: 'globalThis',
  },

  // Handle WASM files
  plugins: [
    {
      name: 'wasm-pack-plugin',
      configureServer(server) {
        server.middlewares.use('/wasm', (req, res, next) => {
          if (req.url.endsWith('.wasm')) {
            res.setHeader('Content-Type', 'application/wasm');
          }
          next();
        });
      }
    }
  ]
})
