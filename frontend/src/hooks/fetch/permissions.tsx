import { createMemo, createResource } from "solid-js";
import { isServer } from "solid-js/web";
import { ListAllPermissionsResponse } from "~/bindings/ListAllPermissionsResponse";
import { useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { get } from "~/utils/func";
import { httpRequest } from "~/utils/http-request";
import { MaybeAccessor } from "~/utils/types";

/**
 * Fetches the static list of all available permission definitions.
 * This list is workspace-independent (the same regardless of which workspace is queried)
 * and cached in localStorage since it never changes unless the database is recreated.
 */
const useFetchPermissions = (workspaceId: MaybeAccessor<string>) => {
	const [authState] = useAuthState();
	const toast = useToast();

	const fetchParams = createMemo(() => {
		return [authState(), get(workspaceId)] as const;
	});

	return createResource(fetchParams, async ([auth, wsId]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn") {
			return { permissions: [] };
		}

		const cacheKey = `all-permissions`;
		if (!isServer) {
			const cached = localStorage.getItem(cacheKey);
			if (cached) {
				try {
					return JSON.parse(cached) as ListAllPermissionsResponse;
				} catch {
					localStorage.removeItem(cacheKey);
				}
			}
		}

		try {
			const response = await httpRequest<ListAllPermissionsResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/permission`,
				{
					method: "GET",
				}
			);

			if (!response.ok) {
				console.error("Failed to fetch permissions:", response.data.error);
				toast(
					"Failed to fetch permissions. Please ensure permissions are properly configured in the database.",
					"error"
				);
				return { permissions: [] };
			}

			if (!isServer) {
				localStorage.setItem(cacheKey, JSON.stringify(response.data));
			}
			return response.data;
		} catch (error) {
			console.error("Error fetching permissions:", error);
			toast("Failed to load permissions", "error");
			return { permissions: [] };
		}
	});
};

export default useFetchPermissions;
