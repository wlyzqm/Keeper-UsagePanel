import { defineConfig } from "vite";
import { readFileSync } from "node:fs";

const { version } = JSON.parse(
  readFileSync(new URL("./package.json", import.meta.url), "utf8"),
);
export default defineConfig({
  define: { __APP_VERSION__: JSON.stringify(version) },
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
