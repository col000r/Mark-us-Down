import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { tauri } from 'vite-plugin-tauri';

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react(), tauri()],
  
  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 3200,
    strictPort: true,
  },
  // 3. to make use of `TAURI_DEBUG` and other env variables
  // https://tauri.studio/v1/api/config#buildconfig.beforedevcommand
  envPrefix: ["VITE_", "TAURI_"],
  
  // Optimize build output
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          'monaco': ['monaco-editor'],
          'vendor': ['react', 'react-dom'],
        },
      },
    },
    chunkSizeWarningLimit: 1000, // 1MB warning limit
  },
});