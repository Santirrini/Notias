/// <reference types="vitest/config" />
import { defineConfig } from "vitest/config";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const __dirname = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  test: {
    include: ["tests/**/*.test.ts"],
    environment: "node",
  },
  resolve: {
    alias: {
      // $lib → src/lib (Vite alias uses regex on the second form).
      $lib: resolve(__dirname, "src/lib"),
      "$lib/(.*)": resolve(__dirname, "src/lib/$1"),
    },
  },
});
