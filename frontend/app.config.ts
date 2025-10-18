import { defineConfig } from "@solidjs/start/config";
import tailwindcss from "@tailwindcss/vite";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  vite: ({ router }) => ({
    plugins: [tailwindcss()],
    publicDir: resolve(__dirname, "../assets/frontend/public"),
    server: {
      fs: {
        allow: [resolve(__dirname, "../assets/frontend")]
      },
      hmr:
        router === "client"
          ? {
            protocol: "ws",
            port: 22300,
            clientPort: 22300,
            path: "hmr/",
          }
          : {},
    },
  }),
});
