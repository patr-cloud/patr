import { defineConfig } from "@solidjs/start/config";
import tailwindcss from "@tailwindcss/vite";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import devtools from "solid-devtools/vite";

const __dirname = dirname(fileURLToPath(import.meta.url));

// `@secretlint/secretlint-rule-preset-recommend` imports `node:path` at the top
// of its bundle and `node:fs` lazily, neither of which a browser build can
// resolve. Point them at stubs — but only for the client router, so the SSR
// bundle keeps the real modules. See `src/utils/node-shims/path.ts`.
const clientNodeShims = {
	"node:path": resolve(__dirname, "./src/utils/node-shims/path.ts"),
	"node:fs": resolve(__dirname, "./src/utils/node-shims/fs.ts"),
};

export default defineConfig({
	middleware: "./src/middleware.ts",
	publicDir: resolve(__dirname, "../assets/frontend/public"),
	vite: ({ router }: { router: any }) => ({
		plugins: [
			tanstackRouter({
				target: "solid",
				autoCodeSplitting: true,
				routesDirectory: "./src/routes",
				generatedRouteTree: "./src/routeTree.gen.ts",
			}),
			tailwindcss(),
			devtools({
				autoname: true,
			}),
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
	}),
});
