import { defineConfig } from 'vite';
import { resolve } from 'path';
import obfuscatorPlugin from 'vite-plugin-javascript-obfuscator';

export default defineConfig({
  // Renderer root: allows imports like ./style.css to resolve correctly
  root: resolve(__dirname, 'src/renderer'),
  base: './',
  build: {
    outDir: resolve(__dirname, 'dist'),
    emptyOutDir: true,
    rollupOptions: {
      input: {
        overlay:  resolve(__dirname, 'src/renderer/overlay/index.html'),
        settings: resolve(__dirname, 'src/renderer/settings/index.html')
      }
    }
  },
  plugins: process.env.NODE_ENV === 'production'
    ? [
        obfuscatorPlugin({
          options: {
            compact: true,
            identifierNamesGenerator: 'hexadecimal',
            controlFlowFlattening: true,
            controlFlowFlatteningThreshold: 0.75,
            deadCodeInjection: true,
            deadCodeInjectionThreshold: 0.4,
            stringArray: true,
            stringArrayEncoding: ['rc4'],
            stringArrayThreshold: 0.85,
            splitStrings: true,
            splitStringsChunkLength: 5,
            numbersToExpressions: true,
            transformObjectKeys: true,
            renameGlobals: false,
            selfDefending: false,
          }
        })
      ]
    : [],
  server: {
    port: 1420,
    strictPort: true
  }
});
