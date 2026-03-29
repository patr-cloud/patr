import { createQuery } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
import { ListAllPermissionsResponse } from "~/bindings/ListAllPermissionsResponse";
import { useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { permissionKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

const usePermissionsQuery = (workspaceId: Accessor<string>) => {
	const [authState] = useAuthState();
	const toast = useToast();

	return createQuery<ListAllPermissionsResponse>(() => {
		const auth = authState();
		const wsId = workspaceId();
		return {
			queryKey: permissionKeys.list(wsId),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn",
			queryFn: async () => {
				const response = await httpRequest<ListAllPermissionsResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/rbac/permission`,
					{ method: "GET" }
				);

				if (!response.ok) {
					toast("Failed to fetch permissions", "error");
					throw new Error(response.data.error);
				}

				return response.data;
			},
		};
	});
};

export default usePermissionsQuery;
