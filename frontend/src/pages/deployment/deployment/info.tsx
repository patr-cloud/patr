import { FiChevronDown } from "solid-icons/fi";
import { createMemo, createResource, createSignal, Resource, Setter } from "solid-js";
import { GetDeploymentInfoResponse, ListRunnersForWorkspaceResponse, UpdateDeploymentResponse } from "~/bindings";
import { Button, Input, InputDropdown, InputLabel, InputType, useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import { EventT } from "~/utils/types";
import EnvInput from "~/pages/deployment/env-input";
import PortInput from "~/pages/deployment/port";

interface DeploymentInfoProps {
	deploymentInfo: Resource<GetDeploymentInfoResponse | undefined>;
	mutateDeploymentInfo: Setter<GetDeploymentInfoResponse | undefined>;
	refetchDeploymentInfo: () =>
		| GetDeploymentInfoResponse
		| Promise<GetDeploymentInfoResponse | undefined>
		| null
		| undefined;
}

const DeploymentInfoUpdate = (props: DeploymentInfoProps) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();

	const [, setHasUpdated] = createSignal(false);

	const resourceParamsRunnerList = createMemo(() => {
		return [authState(), workspaceId()] as const;
	});

	const [runnerList] = createResource(resourceParamsRunnerList, async ([auth, wsId]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn") {
			return { runners: [] };
		}
		const response = await httpRequest<ListRunnersForWorkspaceResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/runner`,
			{
				method: "GET",
				headers: {
					"Content-Type": "application/json",
					Authorization: `Bearer ${auth.accessToken}`,
				},
			}
		);

		if (!response.ok) {
			console.error("Failed to fetch runner list:", response.data.error);
			toast("Failed to fetch runner list", "error");
			return { runners: [] };
		}

		return response.data;
	});

	const onSubmitUpdate = async (e: EventT<SubmitEvent, HTMLFormElement>) => {
		e.preventDefault();
		console.log("Update deployment form submitted");
		const auth = authState();
		if (!auth || auth.type !== "LoggedIn") {
			console.error("User not logged in");
			toast("User not logged in", "error");
			return;
		}

		const response = await httpRequest<UpdateDeploymentResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId()}/deployment/${props.deploymentInfo()?.id}`,
			{
				method: "PATCH",
				headers: {
					"Content-Type": "application/json",
					Authorization: `Bearer ${auth.accessToken}`,
				},
				body: JSON.stringify({
					...props.deploymentInfo(),
				}),
			}
		);

		if (!response.ok) {
			console.error("Failed to update deployment:", response.data.error);
			toast("Failed to update deployment", "error");
			props.refetchDeploymentInfo();
			return;
		}

		toast("Deployment updated successfully", "success");
		props.refetchDeploymentInfo();
	};

	return (
		<form onSubmit={onSubmitUpdate} class="flex flex-col gap-6 justify-between w-full flex-1">
			<div class="flex flex-col gap-4 items-start w-full">
				<div class="flex gap-8 items-center w-full">
					<InputLabel parentClass="flex-2" for="deployment-id" label="ID" />
					<Input
						value={props.deploymentInfo.latest?.id}
						disabled={true}
						class="flex-10"
						name="deployment-id"
						placeholder="Deployment ID"
						type={InputType.Text}
					/>
				</div>

				<div class="flex gap-8 items-center w-full">
					<InputLabel parentClass="flex-2" for="deployment-name" label="Name" />
					<Input
						value={props.deploymentInfo.latest?.name}
						onInput={(e) => {
							setHasUpdated(true);
							props.mutateDeploymentInfo((prev) => {
								return prev
									? {
											...prev,
											name: e.currentTarget.value,
										}
									: undefined;
							});
						}}
						class="flex-10"
						name="deployment-name"
						placeholder="Deployment Name"
						type={InputType.Text}
					/>
				</div>

				<div class="flex gap-8 items-center w-full">
					<InputLabel parentClass="flex-2" for="deployment-runner" label="Runner" />

					<InputDropdown
						options={
							runnerList.latest?.runners.map((runner) => ({
								value: runner.id,
								label: runner.name,
							})) ?? []
						}
						endIcon={() => (
							<button>
								<FiChevronDown size={16} />
							</button>
						)}
						value={props.deploymentInfo.latest?.runner ?? ""}
						onSelect={(runnerId) => {
							setHasUpdated(true);
							props.mutateDeploymentInfo((prev) => {
								return prev
									? {
											...prev,
											runner: runnerId,
										}
									: undefined;
							});
						}}
						class="flex-10"
						name="deployment-runner"
						placeholder="Select Runner"
					/>
				</div>

				<div class="flex gap-8 items-center w-full">
					<InputLabel parentClass="flex-2" for="deployment-registry" label="Registry" />
					<div class="flex-10 flex items-center gap-4 w-full">
						<Input
							value={props.deploymentInfo.latest?.registry ?? ""}
							disabled={true}
							class="flex-4"
							name="deployment-registry"
							placeholder="Select Registry"
						/>

						<Input
							class="flex-6"
							placeholder="Image Name"
							type={InputType.Text}
							onInput={(e) => {
								setHasUpdated(true);
								props.mutateDeploymentInfo((prev) => {
									return prev
										? {
												...prev,
												imageName: e.currentTarget.value,
											}
										: undefined;
								});
							}}
							value={(() => {
								const info = props.deploymentInfo.latest;
								if (!info) return "";
								if (info.registry === "registry.patr.cloud") {
									return "repositoryId" in info ? (info.repositoryId as string) : "";
								}
								return "imageName" in info ? info.imageName : "";
							})()}
						/>

						<Input
							class="flex-2"
							placeholder="Image Tag"
							type={InputType.Text}
							value={props.deploymentInfo.latest?.imageTag ?? "N/A"}
							onInput={(e) => {
								setHasUpdated(true);
								props.mutateDeploymentInfo((prev) => {
									return prev
										? {
												...prev,
												imageTag: e.currentTarget.value,
											}
										: undefined;
								});
							}}
						/>
					</div>
				</div>

				<EnvInput
					envList={Object.entries(props.deploymentInfo.latest?.environmentVariables || {}).map(([key, value]) => ({
						key,
						value,
					}))}
					onAdd={(key, value) => {
						setHasUpdated(true);
						props.mutateDeploymentInfo((prev) => {
							return prev
								? {
										...prev,
										environmentVariables: {
											...prev.environmentVariables,
											[key]: value,
										},
									}
								: undefined;
						});
					}}
					onDelete={(key) => {
						setHasUpdated(true);
						props.mutateDeploymentInfo((prev) => {
							if (!prev) return undefined;
							const newEnv = { ...prev.environmentVariables };
							delete newEnv[key];
							return {
								...prev,
								environmentVariables: newEnv,
							};
						});
					}}
				/>

				<PortInput
					onAdd={(key, value) => {
						setHasUpdated(true);
						console.log(key, value);
						props.mutateDeploymentInfo((prev) => {
							return prev
								? {
										...prev,
										ports: {
											...prev.ports,
											[Number(key)]: value,
										},
									}
								: undefined;
						});
					}}
					onDelete={(key) => {
						setHasUpdated(true);
						props.mutateDeploymentInfo((prev) => {
							if (!prev) return undefined;
							const newPorts = { ...prev.ports };
							delete newPorts[Number(key)];
							return {
								...prev,
								ports: newPorts,
							};
						});
					}}
					portList={props.deploymentInfo.latest?.ports || {}}
				/>
			</div>

			<div class="w-full flex justify-end items-center">
				<Button type="submit" variant="contained">
					UPDATE
				</Button>
			</div>
		</form>
	);
};

export default DeploymentInfoUpdate;
