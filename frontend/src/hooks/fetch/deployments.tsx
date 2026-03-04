import { createMemo, createResource } from "solid-js";
import { useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { httpRequest } from "~/utils/http-request";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { ListDeploymentResponse } from "~/bindings";

const useFetchDeployments = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();

	const fetchParams = createMemo(() => {
		return [authState(), workspaceId] as const;
	});

	return createResource(fetchParams, async ([auth, wsId]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn") {
			return { deployments: [] };
		}

		try {
			const response = await httpRequest<ListDeploymentResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/deployment`,
				{
					method: "GET",
				}
			);

			if (!response.ok) {
				console.error("Failed to fetch deployments:", response.data.error);
				toast("Failed to fetch deployments", "error");
				return { deployments: [] };
			}

			return response.data;
		} catch (error) {
			console.error("Error fetching deployments:", error);
			toast("Failed to load deployments", "error");
			return { deployments: [] };
		}
	});
};

export default useFetchDeployments;
