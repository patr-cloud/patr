import { createMemo, createResource } from "solid-js";
import { ListContainerRepositoriesResponse } from "~/bindings";
import { useToast } from "~/components";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";

const useFetchContainerRepositories = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();

	const fetchParams = createMemo(() => {
		return [authState(), workspaceId()] as const;
	});

	const resource = createResource(fetchParams, async ([auth, wsId]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn") {
			return { repositories: [] };
		}
		const response = await httpRequest<ListContainerRepositoriesResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry`,
			{
				method: "GET",
			}
		);

		if (!response.ok) {
			console.error("Failed to fetch container repositories:", response.data.error);
			toast("Failed to fetch container repositories", "error");
			return { repositories: [] };
		}

		return response.data;
	});

	return resource;
};

export default useFetchContainerRepositories;
