import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Builds dashboard assets into target/public with stable filenames for runtime and packaging.
export default defineConfig({
  plugins: [react()],
  build: {
    outDir: "../target/public",
    emptyOutDir: true,
    cssCodeSplit: false,
    rollupOptions: {
      output: {
        entryFileNames: "app.js",
        chunkFileNames: "app.js",
        assetFileNames: (assetInfo) => {
          const assetNames = [...(assetInfo.names ?? []), ...(assetInfo.originalFileNames ?? [])];
          if (assetNames.some((name) => name.endsWith(".css"))) {
            return "styles.css";
          }
          return "assets/[name][extname]";
        }
      }
    }
  }
});
