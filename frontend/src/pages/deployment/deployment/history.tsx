import { useParams } from "@solidjs/router";
import { createMemo, createResource, Show, Suspense } from "solid-js";
import { ListDeploymentDeployHistoryResponse } from "~/bindings/ListDeploymentDeployHistoryResponse";
import { Table, useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";

const DeploymentHistory = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const params = useParams();

	const resourceParamsDeploymentHistory = createMemo(() => {
		return [authState(), workspaceId(), params.id] as const;
	});

	const [deploymentHistory] = createResource(resourceParamsDeploymentHistory, async ([auth, wsId, id]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn" || id === "") {
			console.log("Invalid parameters for fetching deployment history", wsId, auth, id);
			return undefined;
		}
		const response = await httpRequest<ListDeploymentDeployHistoryResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/deployment/${id}/deploy-history`,
			{
				method: "GET",
				headers: {
					"Content-Type": "application/json",
					Authorization: `Bearer ${auth.accessToken}`,
				},
			}
		);
		if (!response.ok) {
			console.error("Failed to fetch deployment history:", response.data.error);
			toast("Failed to fetch deployment history", "error");
			return undefined;
		}

		console.log("Fetched deployment history:", response.data);
		return response.data;
	});

	return (
		<div class="w-full">
			<Suspense fallback={<div class="text-gray-400 text-center py-8">Loading deployment history...</div>}>
				<Show when={deploymentHistory()} fallback={<div>Loading deployment history...</div>}>
					<Table
						column_grids={["flex-2", "flex-1"]}
						headings={["Event", "Log Date"]}
						rows={deploymentHistory()?.deploys || []}
						renderRow={(deploy) => (
							<tr class="table-row">
								<td class="flex-2">
									<span class="truncate">{deploy.imageDigest}</span>
								</td>
								<td class="flex-1">{deploy.created.toLocaleString().split(".")[0]}</td>
							</tr>
						)}
					/>
				</Show>
			</Suspense>
		</div>
	);
};

export default DeploymentHistory;
