import { createQuery } from "@tanstack/solid-query";
import { ListUserWorkspacesResponse } from "~/bindings";
import { useAuthState } from "~/hooks";
import { workspacesKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

const useWorkspacesQuery = () => {
	const [authState] = useAuthState();

	return createQuery<ListUserWorkspacesResponse>(() => {
		const auth = authState();
		return {
			queryKey: workspacesKeys.list(),
			enabled: !!auth && auth.type === "LoggedIn",
			queryFn: async () => {
				const response = await httpRequest<ListUserWorkspacesResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/user/workspaces`,
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

export default useWorkspacesQuery;
