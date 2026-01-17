import { createMemo, createResource } from "solid-js";
import { ListAllPermissionsResponse } from "~/bindings/ListAllPermissionsResponse";
import { useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { parsePermissionName, safelyParseJSON } from "~/utils/func";
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
			const permissions = localStorage.getItem("user-permissions");
			let parsedPermissions = permissions ? safelyParseJSON<ListAllPermissionsResponse>(permissions) : undefined;

			if (!parsedPermissions) {
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
					throw new Error("Failed to fetch permissions from server");
				}

				const permissionsData = response.data;
				localStorage.setItem("user-permissions", JSON.stringify(permissionsData));
				parsedPermissions = permissionsData as unknown as ListAllPermissionsResponse;
			}

			return parsedPermissions;
		} catch (error) {
			console.error("Error fetching permissions:", error);
			toast("Failed to load permissions", "error");
			return { permissions: [] };
		}
	});
};

export default useFetchPermissions;
