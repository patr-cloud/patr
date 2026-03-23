import { defineConfig } from "@solidjs/start/config";
import tailwindcss from "@tailwindcss/vite";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));

const isCSR = process.env.VITE_BUILD_TARGET === "csr";

export default defineConfig({
  ssr: !isCSR,
  publicDir: resolve(__dirname, "../assets/frontend/public"),
  server: isCSR ? ({
    preset: "static",
  }) : undefined,
  vite: ({ router }: { router: any }) => isCSR ? ({
    plugins: [tanstackRouter({ target: "solid", autoCodeSplitting: true, routesDirectory: "./src/routes", generatedRouteTree: "./src/routeTree.gen.ts" }), tailwindcss()],
    esbuild: {
      pure: ['console.log'],
    },
    // @solid-primitives/storage has an internal circular dependency
    // (cookies.js <-> index.js via a dead import). Forcing the package
    // into a single chunk keeps the cycle chunk-internal, which Rollup
    // handles fine. Only applied to the client router — the SSR router
    // externalizes node_modules so manualChunks can't include them.
    ...(router === "client" ? {
      build: {
        rollupOptions: {
          output: {
            manualChunks: {
              'solid-primitives-storage': ['@solid-primitives/storage'],
            },
          },
        },
      },
    } : {}),
  }) : ({
    plugins: [tanstackRouter({ target: "solid", autoCodeSplitting: true, routesDirectory: "./src/routes", generatedRouteTree: "./src/routeTree.gen.ts" }), tailwindcss()],
    esbuild: {
      pure: ['console.log'],
    },
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
