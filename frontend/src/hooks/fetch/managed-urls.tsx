import { createQuery } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
import { ListManagedURLResponse } from "~/bindings";

import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { managedUrlKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

export const useManagedUrlsQuery = (
	domainId: Accessor<string>,
	page: Accessor<string | undefined>,
	count: Accessor<string | undefined>
) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		const dId = domainId();
		const p = page();
		const c = count();
		return {
			queryKey: managedUrlKeys.page(wsId ?? "", dId, p, c),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!dId,
			meta: { errorMessage: "Failed to fetch managed URLs" },
			queryFn: async () => {
				const params = new URLSearchParams();
				if (p) params.set("page", p);
				if (c) params.set("count", c);
				const qs = params.size > 0 ? `&${params.toString()}` : "";

				const response = await httpRequest<ListManagedURLResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/infrastructure/managed-url?search[domainId]=${dId}${qs}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return {
					urls: response.data.urls,
					totalCount: Number(response.headers.get("x-total-count") ?? 0),
				};
			},
		};
	});
};
