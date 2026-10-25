import { createQuery } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
import { ListOAuthGrantsResponse } from "~/bindings";

import { useAuthState } from "~/hooks/state-hooks";
import { oauthGrantKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

export const useOAuthGrantsQuery = (page: Accessor<string | undefined>, count: Accessor<string | undefined>) => {
	const [authState] = useAuthState();

	return createQuery(() => {
		const auth = authState();
		const p = page();
		const c = count();
		return {
			queryKey: oauthGrantKeys.list(p, c),
			enabled: !!auth && auth.type === "LoggedIn",
			meta: { errorMessage: "Failed to fetch authorized apps" },
			queryFn: async () => {
				const params = new URLSearchParams();
				if (p) params.set("page", p);
				if (c) params.set("count", c);
				const qs = params.size > 0 ? `?${params.toString()}` : "";

				const response = await httpRequest<ListOAuthGrantsResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/user/oauth-grant${qs}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return {
					grants: response.data.grants,
					totalCount: Number(response.headers.get("x-total-count") ?? 0),
				};
			},
		};
	});
};
