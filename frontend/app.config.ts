import { defineConfig } from "@solidjs/start/config";
import tailwindcss from "@tailwindcss/vite";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));

const isCSR = process.env.VITE_BUILD_TARGET === "csr";

// `@secretlint/secretlint-rule-preset-recommend` imports `node:path` at the top
// of its bundle and `node:fs` lazily, neither of which a browser build can
// resolve. Point them at stubs — but only for the client router, so the SSR
// bundle keeps the real modules. See `src/utils/node-shims/path.ts`.
const clientNodeShims = {
	"node:path": resolve(__dirname, "./src/utils/node-shims/path.ts"),
	"node:fs": resolve(__dirname, "./src/utils/node-shims/fs.ts"),
};

export default defineConfig({
	ssr: !isCSR,
	middleware: isCSR ? undefined : "./src/middleware.ts",
	publicDir: resolve(__dirname, "../assets/frontend/public"),
	server: isCSR
		? {
			preset: "static",
		}
		: undefined,
	vite: ({ router }: { router: any }) =>
		isCSR
			? {
				plugins: [
					tanstackRouter({
						target: "solid",
						autoCodeSplitting: true,
						routesDirectory: "./src/routes",
						generatedRouteTree: "./src/routeTree.gen.ts",
					}),
					tailwindcss(),
				],
				esbuild: {
					pure: ["console.log"],
				},
				...(router === "client" ? { resolve: { alias: clientNodeShims } } : {}),
				// @solid-primitives/storage has an internal circular dependency
				// (cookies.js <-> index.js via a dead import). Forcing the package
				// into a single chunk keeps the cycle chunk-internal, which Rollup
				// handles fine. Only applied to the client router — the SSR router
				// externalizes node_modules so manualChunks can't include them.
				...(router === "client"
					? {
						build: {
							rollupOptions: {
								onwarn(warning: any, warn: any) {
									if (warning.code === "CYCLIC_CROSS_CHUNK_REEXPORT") return;
									warn(warning);
								},
								output: {
									manualChunks: {
										"solid-primitives-storage": ["@solid-primitives/storage"],
									},
								},
							},
						},
					}
					: {}),
			}
			: {
				plugins: [
					tanstackRouter({
						target: "solid",
						autoCodeSplitting: true,
						routesDirectory: "./src/routes",
						generatedRouteTree: "./src/routeTree.gen.ts",
					}),
					tailwindcss(),
				],
				esbuild: {
					pure: ["console.log"],
				},
				...(router === "client" ? { resolve: { alias: clientNodeShims } } : {}),
				server: {
					fs: {
						allow: [resolve(__dirname, "../assets/frontend")],
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
			},
});
