import { createQuery } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
import { GetWorkspaceInfoResponse } from "~/bindings/GetWorkspaceInfoResponse";
import { useToast } from "~/components";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { workspaceKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

export const useWorkspaceInfoQuery = (workspaceId?: Accessor<string | undefined>) => {
	const [authState] = useAuthState();
	const [lastWorkspaceId] = useLastWorkspaceId();
	const toast = useToast();

	return createQuery<GetWorkspaceInfoResponse>(() => {
		const auth = authState();
		const wsId = workspaceId ? workspaceId() : lastWorkspaceId();
		return {
			queryKey: workspaceKeys.info(wsId ?? ""),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn",
			queryFn: async () => {
				const response = await httpRequest<GetWorkspaceInfoResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					toast("Failed to fetch workspace info", "error");
					throw new Error(response.data.error);
				}

				return response.data;
			},
		};
	});
};
