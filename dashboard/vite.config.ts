import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Builds the dashboard into the Rust static directory with stable filenames.
export default defineConfig({
  plugins: [react()],
  build: {
    outDir: "../crates/server/static",
    emptyOutDir: true,
    cssCodeSplit: false,
    rollupOptions: {
      output: {
        entryFileNames: "app.js",
        chunkFileNames: "app.js",
        assetFileNames: (assetInfo) => {
          if (assetInfo.name?.endsWith(".css")) {
            return "styles.css";
          }
          return "assets/[name][extname]";
        }
      }
    }
  }
});
