import { createMemo, createResource } from "solid-js";
import { ListAllPermissionsResponse } from "~/bindings/ListAllPermissionsResponse";
import { useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { httpRequest } from "~/utils/http-request";

const useFetchPermissions = (workspaceId?: string) => {
	const [authState] = useAuthState();
	const toast = useToast();

	const fetchParams = createMemo(() => {
		return [authState(), workspaceId] as const;
	});

	return createResource(fetchParams, async ([auth, wsId]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn") {
			return { permissions: [] };
		}

		try {
			const response = await httpRequest<ListAllPermissionsResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/permission`,
				{
					method: "GET",
					headers: {
						"Content-Type": "application/json",
						Authorization: `Bearer ${auth.accessToken}`,
					},
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

			return response.data;
		} catch (error) {
			console.error("Error fetching permissions:", error);
			toast("Failed to load permissions", "error");
			return { permissions: [] };
		}
	});
};

export default useFetchPermissions;
