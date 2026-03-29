import { createQuery } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
import { ListApiTokensResponse } from "~/bindings";
import { useToast } from "~/components";
import { useAuthState } from "~/hooks/state-hooks";
import { apiTokenKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

export const useApiTokensQuery = (page: Accessor<string | undefined>, count: Accessor<string | undefined>) => {
	const [authState] = useAuthState();
	const toast = useToast();

	return createQuery(() => {
		const auth = authState();
		const p = page();
		const c = count();
		return {
			queryKey: apiTokenKeys.list(p, c),
			enabled: !!auth && auth.type === "LoggedIn",
			queryFn: async () => {
				const params = new URLSearchParams();
				if (p) params.set("page", p);
				if (c) params.set("count", c);
				const qs = params.size > 0 ? `?${params.toString()}` : "";

				const response = await httpRequest<ListApiTokensResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/user/api-token${qs}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					toast("Failed to fetch API tokens", "error");
					throw new Error(response.data.error);
				}

				return {
					tokens: response.data.tokens,
					totalCount: Number(response.headers.get("x-total-count") ?? 0),
				};
			},
		};
	});
};
