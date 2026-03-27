import { FiChevronDown } from "solid-icons/fi";
import { createSignal, Setter, Show } from "solid-js";
import { GetDeploymentInfoResponse, UpdateDeploymentResponse } from "~/bindings";
import {
	Button,
	CopyableField,
	CopyableFieldVariant,
	Input,
	InputType,
	InputDropdown,
	InputLabel,
	RangeSlider,
	ToggleSwitch,
	useToast,
} from "~/components";
import { useAuthState } from "~/hooks";
import { useGetPermissions } from "~/hooks/is-allowed";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { useRunnersQuery } from "~/hooks/fetch";
import { httpRequest } from "~/utils/http-request";
import { EventT } from "~/utils/types";
import EnvInput from "./env-input";
import PortInput from "./port";

interface DeploymentInfoProps {
	deploymentInfo: GetDeploymentInfoResponse | undefined;
	mutateDeploymentInfo: Setter<GetDeploymentInfoResponse | undefined>;
	refetchDeploymentInfo: () => void;
}

const PATR_REGISTRY = "registry.patr.cloud";

const DeploymentInfoUpdate = (props: DeploymentInfoProps) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const deploymentPermissions = useGetPermissions("deployment", () => props.deploymentInfo?.id || "");

	const [_, setHasUpdated] = createSignal(false);

	const runnersQuery = useRunnersQuery();

	const isPatrRegistry = () => {
		const info = props.deploymentInfo.latest;
		if (!info) return false;
		return info.registry === PATR_REGISTRY;
	};

	const onSubmitUpdate = async (e: EventT<SubmitEvent, HTMLFormElement>) => {
		e.preventDefault();
		const auth = authState();
		if (!auth || auth.type !== "LoggedIn") {
			toast("User not logged in", "error");
			return;
		}

		const info = props.deploymentInfo();
		if (!info) {
			toast("Deployment info not available", "error");
			return;
		}

		const response = await httpRequest<UpdateDeploymentResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId()}/deployment/${info.id}`,
			{
				method: "PATCH",
				body: JSON.stringify({
					name: info.name,
					runner: info.runner,
					deployOnPush: info.deployOnPush,
					minHorizontalScale: info.minHorizontalScale,
					maxHorizontalScale: info.maxHorizontalScale,
					ports: info.ports,
					environmentVariables: info.environmentVariables,
					startupProbe: info.startupProbe,
					livenessProbe: info.livenessProbe,
					configMounts: info.configMounts,
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
				<div class="flex gap-8 items-start w-full">
					<InputLabel parentClass="flex-2 pt-2.5" for="deployment-id" label="ID" />
					<CopyableField
						value={props.deploymentInfo?.id ?? ""}
						variant={CopyableFieldVariant.Input}
						class="flex-10"
					/>
				</div>

				<div class="flex gap-8 items-start w-full">
					<InputLabel parentClass="flex-2 pt-2.5" for="deployment-name" label="Name" />
					<Input
						class="flex-10"
						name="deployment-name"
						placeholder="Deployment Name"
						type={InputType.Text}
						disabled={!deploymentPermissions().edit}
						value={props.deploymentInfo?.name}
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
					/>
				</div>

				<div class="flex gap-8 items-start w-full">
					<InputLabel
						parentClass="flex-2 pt-2.5"
						label="Current Digest"
						comments="Image hash running in production"
					/>
					<div class="flex-10">
						<Show
							when={props.deploymentInfo.latest?.currentLiveDigest}
							fallback={<Input disabled={true} placeholder="No digest available" type={InputType.Text} />}
						>
							<CopyableField
								value={props.deploymentInfo.latest!.currentLiveDigest!}
								variant={CopyableFieldVariant.Input}
								class="font-log"
							/>
						</Show>
					</div>
				</div>

				<div class="flex gap-8 items-start w-full">
					<InputLabel parentClass="flex-2 pt-2.5" for="deployment-runner" label="Runner" />

					<InputDropdown
						class="flex-10"
						name="deployment-runner"
						placeholder="Select Runner"
						disabled={!deploymentPermissions().edit}
						value={props.deploymentInfo?.runner ?? ""}
						endIcon={() => (
							<button>
								<FiChevronDown size={16} />
							</button>
						)}
						options={
							runnersQuery.data?.runners.map((runner) => ({
								value: runner.id,
								label: runner.name,
							})) ?? []
						}
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
					/>
				</div>

				<div class="flex gap-8 items-start w-full">
					<InputLabel parentClass="flex-2 pt-2.5" for="deployment-registry" label="Image" />
					<div class="flex-10 flex items-center gap-4 w-full">
						<Input
							value={props.deploymentInfo?.registry ?? ""}
							disabled={true}
							class="flex-4"
							name="deployment-registry"
							placeholder="Select Registry"
						/>

						<Input
							disabled={true}
							class="flex-6"
							placeholder="Image Name"
							type={InputType.Text}
							value={(() => {
								const info = props.deploymentInfo;
								if (!info) return "";
								if (info.registry === PATR_REGISTRY) {
									return "repositoryId" in info ? (info.repositoryId as string) : "";
								}
								return "imageName" in info ? info.imageName : "";
							})()}
						/>

						<Input
							class="flex-2"
							disabled={!deploymentPermissions().edit}
							placeholder="Image Tag"
							type={InputType.Text}
							value={props.deploymentInfo?.imageTag ?? "N/A"}
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

				{/* Divider */}
				<div class="border-t border-border-color w-full mt-2" />

				<div class="flex gap-8 items-center w-full">
					<InputLabel parentClass="flex-2" label="Horizontal Scale" comments="Min & max replica count" />
					<div class="flex-10">
						<RangeSlider
							min={1}
							max={10}
							valueLow={() => props.deploymentInfo.latest?.minHorizontalScale ?? 1}
							valueHigh={() => props.deploymentInfo.latest?.maxHorizontalScale ?? 2}
							disabled={!deploymentPermissions().edit}
							onChangeLow={(val) => {
								setHasUpdated(true);
								props.mutateDeploymentInfo((prev) =>
									prev ? { ...prev, minHorizontalScale: val } : undefined
								);
							}}
							onChangeHigh={(val) => {
								setHasUpdated(true);
								props.mutateDeploymentInfo((prev) =>
									prev ? { ...prev, maxHorizontalScale: val } : undefined
								);
							}}
						/>
					</div>
				</div>

				<Show when={isPatrRegistry()}>
					<div class="flex gap-8 items-center w-full">
						<InputLabel parentClass="flex-2" label="Deploy on Push" comments="Redeploy on new image push" />
						<div class="flex-10">
							<ToggleSwitch
								checked={() => props.deploymentInfo.latest?.deployOnPush ?? false}
								disabled={!deploymentPermissions().edit}
								onChange={(val) => {
									setHasUpdated(true);
									props.mutateDeploymentInfo((prev) =>
										prev ? { ...prev, deployOnPush: val } : undefined
									);
								}}
							/>
						</div>
					</div>
				</Show>

				{/* Divider */}
				<div class="border-t border-border-color w-full mt-2" />

				<EnvInput
					disabled={() => !deploymentPermissions().edit}
					envList={Object.entries(props.deploymentInfo?.environmentVariables || {}).map(([key, value]) => ({
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
					disabled={() => !deploymentPermissions().edit}
					portList={props.deploymentInfo?.ports || {}}
					deploymentId={props.deploymentInfo?.id}
					onAdd={(key, value) => {
						setHasUpdated(true);
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
				/>
			</div>

			<Show when={deploymentPermissions().edit}>
				<div class="w-full flex justify-end items-center">
					<Button disabled={!deploymentPermissions().edit} type="submit" variant="contained">
						Update
					</Button>
				</div>
			</Show>
		</form>
	);
};

export default DeploymentInfoUpdate;
