import { defineConfig } from "@solidjs/start/config";
import tailwindcss from "@tailwindcss/vite";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));

const isCSR = process.env.VITE_BUILD_TARGET === "csr";

export default defineConfig({
  ssr: !isCSR,
  server: isCSR ? ({
    preset: "static",
  }) : undefined,
  vite: ({ router }: { router: any }) => isCSR ? ({
    plugins: [tanstackRouter({ target: "solid", autoCodeSplitting: true, routesDirectory: "./src/routes", generatedRouteTree: "./src/routeTree.gen.ts" }), tailwindcss()],
    esbuild: {
      pure: ['console.log'],
    },
  }) : ({
    plugins: [tanstackRouter({ target: "solid", autoCodeSplitting: true, routesDirectory: "./src/routes", generatedRouteTree: "./src/routeTree.gen.ts" }), tailwindcss()],
    esbuild: {
      pure: ['console.log'],
    },
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
