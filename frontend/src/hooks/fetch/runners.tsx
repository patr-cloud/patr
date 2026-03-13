import { createMemo, createResource } from "solid-js";
import { ListRunnersForWorkspaceResponse } from "~/bindings";
import { useToast } from "~/components";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";

const useFetchRunners = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();

	const fetchParams = createMemo(() => {
		return [authState(), workspaceId()] as const;
	});

	const resource = createResource(fetchParams, async ([auth, wsId]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn") {
			return { runners: [] };
		}
		const response = await httpRequest<ListRunnersForWorkspaceResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/runner`,
			{
				method: "GET",
			}
		);

		if (!response.ok) {
			console.error("Failed to fetch runners:", response.data.error);
			toast("Failed to fetch runners", "error");
			return { runners: [] };
		}

		return response.data;
	});

	return resource;
};

export default useFetchRunners;
