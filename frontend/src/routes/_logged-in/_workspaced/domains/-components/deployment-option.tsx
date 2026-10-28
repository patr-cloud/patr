import { Show } from "solid-js";
import { InputDropdown } from "~/components";
import { useDeploymentsQuery, useDeploymentInfoQuery } from "~/hooks/fetch";

type DeploymentOptionProps = {
	deployment: string | null;
	onSelectDeployment: (value: string) => void;
	port: number | null;
	onPortChange: (value: number) => void;
};

const DeploymentOption = (props: DeploymentOptionProps) => {
	const deploymentsQuery = useDeploymentsQuery(
		() => undefined,
		() => undefined
	);

	const deploymentInfoQuery = useDeploymentInfoQuery(() => props.deployment || "");

	// Guarded on status rather than reading `data` straight: picking a deployment
	// re-keys the info query, and a pending read would suspend the whole route,
	// blanking the page and scrolling it to the top.
	const deployments = () => (deploymentsQuery.isSuccess ? deploymentsQuery.data.deployments : []);
	const deploymentInfo = () => (deploymentInfoQuery.isSuccess ? deploymentInfoQuery.data : undefined);

	return (
		<div class="flex items-center gap-2 flex-4">
			<InputDropdown
				class={deploymentInfo() ? `flex-2` : `flex-4`}
				onSelect={(value) => {
					props.onSelectDeployment(value);
				}}
				styleVariant="medium"
				placeholder="Select deployment"
				value={props.deployment || undefined}
				options={deployments().map((deployment) => ({
					value: deployment.id,
					label: deployment.name,
				}))}
			/>

			<Show when={deploymentInfo()}>
				<InputDropdown
					class="flex-2"
					styleVariant="medium"
					placeholder="Port"
					onSelect={(value) => {
						props.onPortChange(Number(value));
					}}
					value={String(props.port) || undefined}
					options={
						Object.keys(deploymentInfo()?.ports || {}).map((port) => ({
							value: String(port),
							label: `${port}`,
						})) || []
					}
				/>
			</Show>
		</div>
	);
};

export default DeploymentOption;
