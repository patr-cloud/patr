import { createMemo, createResource, Suspense } from "solid-js";
import { GetDeploymentInfoResponse, ListDeploymentResponse } from "~/bindings";
import { InputDropdown } from "~/components";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";

type DeploymentOptionProps = {
	deployment: string | null;
	onSelectDeployment: (value: string) => void;
	port: number | null;
	onPortChange: (value: number) => void;
};

const DeploymentOption = (props: DeploymentOptionProps) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	const fetchParams = createMemo(() => {
		return [authState(), workspaceId()] as const;
	});

	const [deployments] = createResource(fetchParams, async ([auth, wsId]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn") {
			return { deployments: [] };
		}

		const response = await httpRequest<ListDeploymentResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/deployment`,
			{
				method: "GET",
			}
		);

		if (!response.ok) {
			console.error("Failed to fetch deployments:", response.data.error);
			return { deployments: [] };
		}

		// Fetch deployments logic goes here
		return { deployments: response.data.deployments };
	});

	const deploymentInfoParams = createMemo(() => {
		return [authState(), workspaceId(), props.deployment] as const;
	});

	const [deploymentInfo] = createResource(deploymentInfoParams, async ([auth, wsId, deploymentId]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn" || !deploymentId) {
			return;
		}

		const response = await httpRequest<GetDeploymentInfoResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/deployment/${deploymentId}`,
			{
				method: "GET",
				headers: {
					Authorization: `Bearer ${auth.accessToken}`,
				},
			}
		);

		if (!response.ok) {
			console.error("Failed to fetch deployment info:", response.data.error);
			return;
		}

		console.log("Fetched deployment info:", response.data);

		return response.data;
	});

	return (
		<Suspense fallback={<div>Loading deployments...</div>}>
			<div class="flex items-center gap-2 flex-4">
				<InputDropdown
					class={deploymentInfo.latest ? `flex-2` : `flex-4`}
					onSelect={(value) => {
						props.onSelectDeployment(value);
					}}
					styleVariant="medium"
					value={props.deployment || undefined}
					options={
						deployments.latest?.deployments.map((deployment) => ({
							value: deployment.id,
							label: deployment.name,
						})) || []
					}
				/>

				<Suspense fallback={<div class="mt-2 text-sm text-gray-500">Loading deployment info...</div>}>
					{deploymentInfo.latest && (
						<InputDropdown
							class="flex-2"
							styleVariant="medium"
							onSelect={(value) => {
								props.onPortChange(Number(value));
							}}
							value={String(props.port) || undefined}
							options={
								Object.keys(deploymentInfo.latest?.ports || {}).map((port) => ({
									value: String(port),
									label: `${port}`,
								})) || []
							}
						/>
					)}
				</Suspense>
			</div>
		</Suspense>
	);
};

export default DeploymentOption;
