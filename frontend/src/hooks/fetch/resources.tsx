import { createQuery } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
import { GetResourcesInfoRequest } from "~/bindings/GetResourcesInfoRequest";
import { GetResourcesInfoResponse } from "~/bindings/GetResourcesInfoResponse";
import { ResourceInfo } from "~/bindings/ResourceInfo";

import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { resourceKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

/**
 * Resolves a batch of resource IDs into their names and resource types.
 *
 * Permissions only store resource IDs, so anything that needs to display them
 * (the role permissions table, for instance) has to resolve them first. The IDs
 * are sorted before use so that the same set of resources — in any order — hits
 * the same cache entry.
 *
 * IDs that don't resolve come back from the API with `null` fields rather than
 * being omitted (a deleted resource, or one whose type has no name), so the map
 * always has an entry for every ID that was asked for. Callers should fall back
 * to showing the raw ID in that case.
 */
const useResourcesInfoQuery = (resourceIds: Accessor<string[]>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		const ids = [...resourceIds()].sort();
		return {
			queryKey: resourceKeys.info(wsId ?? "", ids),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && ids.length > 0,
			meta: { errorMessage: "Failed to fetch resource details" },
			queryFn: async () => {
				const body: GetResourcesInfoRequest = { resourceIds: ids };
				const response = await httpRequest<GetResourcesInfoResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/resources-info`,
					{ method: "POST", body: JSON.stringify(body) }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return new Map<string, ResourceInfo>(
					(response.data.resources || []).map(({ id, ...info }) => [id, info])
				);
			},
		};
	});
};

export default useResourcesInfoQuery;
