import { createQuery } from "@tanstack/solid-query";
import { GetApiEnvironmentResponse } from "~/bindings";

import { apiEnvironmentKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

/**
 * Fetches `/api/info` once and caches it for the session. Used to read
 * instance-level values the bundle can't know at build time — currently
 * the self-hosted base domain. Cached effectively forever via `staleTime:
 * Infinity` since the response only changes on a redeploy.
 */
const useApiEnvironmentQuery = () =>
	createQuery<GetApiEnvironmentResponse>(() => ({
		queryKey: apiEnvironmentKeys.all(),
		staleTime: Infinity,
		gcTime: Infinity,
		meta: { errorMessage: "Failed to fetch API environment" },
		queryFn: async () => {
			const response = await httpRequest<GetApiEnvironmentResponse>(`${import.meta.env.VITE_BASE_URL}/api/info`, {
				method: "GET",
			});
			if (!response.ok) {
				throw new Error("Failed to fetch API environment");
			}
			return response.data;
		},
	}));

export default useApiEnvironmentQuery;
