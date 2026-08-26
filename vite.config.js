import { defineConfig } from "vite";
export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: [
        "**/.tools/**",
        "**/.cache/**",
        "**/target/**",
        "**/desktop/**",
        "**/dist/**",
      ],
    },
  },
  build: { outDir: "web-dist", target: "es2022" },
});
