import { notFound } from "@tanstack/solid-router";

export const IS_CLOUD = import.meta.env.VITE_CLOUD_MODE === "true";

export const BASE_DOMAIN: string | undefined =
	import.meta.env.VITE_BASE_DOMAIN ?? (IS_CLOUD ? "patr.cloud" : undefined);

export const DEPLOYMENT_DOMAIN: string | undefined =
	import.meta.env.VITE_DEPLOYMENT_DOMAIN ?? (IS_CLOUD ? "onpatr.cloud" : undefined);

export const REGISTRY_DOMAIN: string | undefined = BASE_DOMAIN ? `registry.${BASE_DOMAIN}` : undefined;

export function cloudOnly<T>(config: T): T {
	if (IS_CLOUD) {
		return config;
	}
	return {
		beforeLoad: () => {
			throw notFound();
		},
	} as T;
}
