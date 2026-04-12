import { createQuery } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
import { ListManagedURLResponse } from "~/bindings";

import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { managedUrlKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

export const useManagedUrlsQuery = (domainId: Accessor<string>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery<ListManagedURLResponse>(() => {
		const auth = authState();
		const wsId = workspaceId();
		const dId = domainId();
		return {
			queryKey: managedUrlKeys.list(wsId ?? "", dId),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!dId,
			meta: { errorMessage: "Failed to fetch managed URLs" },
			queryFn: async () => {
				const response = await httpRequest<ListManagedURLResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/infrastructure/managed-url?search[domainId]=${dId}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return response.data;
			},
		};
	});
};
