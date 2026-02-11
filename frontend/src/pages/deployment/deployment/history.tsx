import { useParams } from "@solidjs/router";
import { createMemo, createResource, Show, Suspense } from "solid-js";
import { ListDeploymentDeployHistoryResponse } from "~/bindings/ListDeploymentDeployHistoryResponse";
import { Button, ButtonVariant, Table, useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { formatRelativeTime } from "~/utils/func";
import { httpRequest } from "~/utils/http-request";
import { CopyButton } from "../list";

const DeploymentHistory = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const params = useParams();

	const resourceParamsDeploymentHistory = createMemo(() => {
		return [authState(), workspaceId(), params.id] as const;
	});

	const [deploymentHistory, { refetch }] = createResource(resourceParamsDeploymentHistory, async ([auth, wsId, id]) => {
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

	const handleDeploy = async (imageDigest: string) => {
		const auth = authState();
		const wsId = workspaceId();
		const deploymentId = params.id;

		if (!auth || auth.type !== "LoggedIn" || !wsId || !deploymentId) {
			toast("User not logged in or missing parameters", "error");
			return;
		}

		const response = await httpRequest(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/deployment/${deploymentId}`,
			{
				method: "PATCH",
				headers: {
					"Content-Type": "application/json",
					Authorization: `Bearer ${auth.accessToken}`,
				},
				body: JSON.stringify({
					currentLiveDigest: imageDigest,
				}),
			}
		);

		if (!response.ok) {
			console.error("Failed to deploy:", response.data.error);
			toast("Failed to deploy image", "error");
			return;
		}

		toast("Deployment triggered successfully", "success");
		refetch();
	};

	return (
		<div class="w-full">
			<Suspense fallback={<div class="text-gray-400 text-center py-8">Loading deployment history...</div>}>
				<Show when={deploymentHistory()} fallback={<div>Loading deployment history...</div>}>
					<Table
						column_grids={["flex-3", "flex-1", "flex-1"]}
						headings={["Digest", "Pushed At", "Action"]}
						rows={deploymentHistory()?.deploys || []}
						renderRow={(deploy) => (
							<tr class="table-row">
								<td class="flex-3">
									<span class="truncate">{deploy.imageDigest}</span>
									<CopyButton text={deploy.imageDigest} />
								</td>
								<td class="flex-1">{formatRelativeTime(deploy.created)}</td>
								<td class="flex-1">
									<div class="flex py-2">
										<Button
											variant={ButtonVariant.Contained}
											type="button"
											onClick={() => handleDeploy(deploy.imageDigest)}
										>
											Deploy
										</Button>
									</div>
								</td>
							</tr>
						)}
					/>
				</Show>
			</Suspense>
		</div>
	);
};

export default DeploymentHistory;
