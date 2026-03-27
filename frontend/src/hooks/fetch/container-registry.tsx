import { createQuery } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
import { ListContainerRepositoriesResponse } from "~/bindings";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { containerRegistryKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

export const useContainerRegistriesQuery = (
	page: Accessor<string | undefined>,
	count: Accessor<string | undefined>
) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		const p = page();
		const c = count();
		return {
			queryKey: containerRegistryKeys.list(wsId ?? "", p, c),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn",
			queryFn: async () => {
				const params = new URLSearchParams();
				if (p) params.set("page", p);
				if (c) params.set("count", c);
				const qs = params.size > 0 ? `?${params.toString()}` : "";

				const response = await httpRequest<ListContainerRepositoriesResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry${qs}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return {
					repositories: response.data.repositories,
					totalCount: Number(response.headers.get("x-total-count") ?? 0),
				};
			},
		};
	});
};
