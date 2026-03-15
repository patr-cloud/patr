import { createRouter, createMemoryHistory } from "@tanstack/solid-router";
import { routeTree } from "./routeTree.gen";
import { AuthState } from "./hooks/state-hooks";
import { isServer, getRequestEvent } from "solid-js/web";

export interface RouterContext {
	auth: AuthState | null;
}

function getServerUrl(): string {
	const event = getRequestEvent();
	if (event?.request?.url) {
		const url = new URL(event.request.url);
		return url.pathname + url.search;
	}
	return "/";
}

export function createAppRouter() {
	return createRouter({
		routeTree,
		context: {
			auth: undefined!,
		},
		...(isServer
			? {
					history: createMemoryHistory({
						initialEntries: [getServerUrl()],
					}),
				}
			: {}),
	});
}

// Type registration - uses ReturnType to avoid calling createRouter at module scope
declare module "@tanstack/solid-router" {
	interface Register {
		router: ReturnType<typeof createAppRouter>;
	}
}
