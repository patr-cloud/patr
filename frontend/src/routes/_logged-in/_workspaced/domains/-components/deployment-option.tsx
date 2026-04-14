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

	return (
		<div class="flex items-center gap-2 flex-4">
			<InputDropdown
				class={deploymentInfoQuery.data ? `flex-2` : `flex-4`}
				onSelect={(value) => {
					props.onSelectDeployment(value);
				}}
				styleVariant="medium"
				value={props.deployment || undefined}
				options={
					deploymentsQuery.data?.deployments.map((deployment) => ({
						value: deployment.id,
						label: deployment.name,
					})) || []
				}
			/>

			<Show when={deploymentInfoQuery.data}>
				<InputDropdown
					class="flex-2"
					styleVariant="medium"
					onSelect={(value) => {
						props.onPortChange(Number(value));
					}}
					value={String(props.port) || undefined}
					options={
						Object.keys(deploymentInfoQuery.data?.ports || {}).map((port) => ({
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
