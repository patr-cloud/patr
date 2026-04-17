import { FiChevronDown } from "solid-icons/fi";
import { createEffect, createSignal, Show } from "solid-js";
import { ExposedPortType, GetDeploymentInfoResponse, UpdateDeploymentResponse } from "~/bindings";
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
import { useDeploymentInfoQuery, useRunnersQuery } from "~/hooks/fetch";
import { deploymentKeys } from "~/hooks/query-keys";
import { useQueryClient } from "@tanstack/solid-query";
import { httpRequest } from "~/utils/http-request";
import { EventT } from "~/utils/types";
import EnvInput from "./env-input";
import PortInput from "./port";
import ConfigMount from "./config-mount";

interface DeploymentInfoProps {
	deploymentId: string;
}

const PATR_REGISTRY = "registry.patr.cloud";

const DeploymentInfoUpdate = (props: DeploymentInfoProps) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const queryClient = useQueryClient();

	const deploymentQuery = useDeploymentInfoQuery(() => props.deploymentId);
	const runnersQuery = useRunnersQuery();

	// Local signal for form editing — initialized from query data and kept in sync
	const [localInfo, setLocalInfo] = createSignal<GetDeploymentInfoResponse | undefined>(undefined);

	createEffect(() => {
		if (deploymentQuery.data && !localInfo()) {
			setLocalInfo(deploymentQuery.data);
		}
	});

	const deploymentPermissions = useGetPermissions("deployment", () => props.deploymentId);

	const [_, setHasUpdated] = createSignal(false);
	const [isUpdating, setIsUpdating] = createSignal(false);
	const [envValid, setEnvValid] = createSignal(true);
	const [portsValid, setPortsValid] = createSignal(true);

	type DeployInfo = GetDeploymentInfoResponse | undefined;
	const updateLocal = (fn: (prev: DeployInfo) => DeployInfo) => {
		setHasUpdated(true);
		setLocalInfo(fn);
	};

	const isPatrRegistry = () => {
		const info = localInfo();
		if (!info) return false;
		return info.registry === PATR_REGISTRY;
	};

	const refetchDeploymentInfo = () => {
		const wsId = workspaceId();
		if (wsId) {
			queryClient.invalidateQueries({ queryKey: deploymentKeys.detail(wsId, props.deploymentId) });
		}
	};

	const onSubmitUpdate = async (e: EventT<SubmitEvent, HTMLFormElement>) => {
		e.preventDefault();
		const auth = authState();
		if (!auth || auth.type !== "LoggedIn") {
			toast("User not logged in", "error");
			return;
		}

		const info = localInfo();
		if (!info) {
			toast("Deployment info not available", "error");
			return;
		}

		setIsUpdating(true);
		try {
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
				refetchDeploymentInfo();
				return;
			}

			toast("Deployment updated successfully", "success");
			refetchDeploymentInfo();
		} finally {
			setIsUpdating(false);
		}
	};

	return (
		<form onSubmit={onSubmitUpdate} class="flex flex-col gap-6 justify-between w-full flex-1">
			<div class="flex flex-col gap-4 items-start w-full">
				<div class="flex gap-8 items-start w-full">
					<InputLabel parentClass="flex-2 pt-2.5" for="deployment-id" label="ID" />
					<CopyableField
						value={localInfo()?.id ?? ""}
						variant={CopyableFieldVariant.Input}
						buttonPosition="start"
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
						value={localInfo()?.name}
						onInput={(e) => {
							updateLocal((prev) => (prev ? { ...prev, name: e.currentTarget.value } : undefined));
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
							when={localInfo()?.currentLiveDigest}
							fallback={<Input disabled={true} placeholder="No digest available" type={InputType.Text} />}
						>
							<CopyableField
								value={localInfo()!.currentLiveDigest!}
								variant={CopyableFieldVariant.Input}
								buttonPosition="start"
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
						value={localInfo()?.runner ?? ""}
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
							updateLocal((prev) => (prev ? { ...prev, runner: runnerId } : undefined));
						}}
					/>
				</div>

				<div class="flex gap-8 items-start w-full">
					<InputLabel parentClass="flex-2 pt-2.5" for="deployment-registry" label="Image" />
					<div class="flex-10 flex items-center gap-4 w-full">
						<Input
							value={localInfo()?.registry ?? ""}
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
								const info = localInfo();
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
							value={localInfo()?.imageTag ?? "N/A"}
							onInput={(e) => {
								updateLocal((prev) =>
									prev ? { ...prev, imageTag: e.currentTarget.value } : undefined
								);
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
							valueLow={() => localInfo()?.minHorizontalScale ?? 1}
							valueHigh={() => localInfo()?.maxHorizontalScale ?? 2}
							disabled={!deploymentPermissions().edit}
							onChangeLow={(val) => {
								updateLocal((prev) => (prev ? { ...prev, minHorizontalScale: val } : undefined));
							}}
							onChangeHigh={(val) => {
								updateLocal((prev) => (prev ? { ...prev, maxHorizontalScale: val } : undefined));
							}}
						/>
					</div>
				</div>

				<Show when={isPatrRegistry()}>
					<div class="flex gap-8 items-center w-full">
						<InputLabel parentClass="flex-2" label="Deploy on Push" comments="Redeploy on new image push" />
						<div class="flex-10">
							<ToggleSwitch
								checked={() => localInfo()?.deployOnPush ?? false}
								disabled={!deploymentPermissions().edit}
								onChange={(val) => {
									updateLocal((prev) => (prev ? { ...prev, deployOnPush: val } : undefined));
								}}
							/>
						</div>
					</div>
				</Show>

				{/* Divider */}
				<div class="border-t border-border-color w-full mt-2" />

				<EnvInput
					disabled={() => !deploymentPermissions().edit}
					value={() => deploymentQuery.data?.environmentVariables ?? {}}
					onChange={(next) =>
						updateLocal((prev) => (prev ? { ...prev, environmentVariables: next } : undefined))
					}
					onValidityChange={setEnvValid}
				/>

				<PortInput
					disabled={() => !deploymentPermissions().edit}
					value={() => (deploymentQuery.data?.ports ?? {}) as Record<string, ExposedPortType>}
					deploymentId={localInfo()?.id}
					onChange={(next) => {
						const numericPorts: Record<number, ExposedPortType> = {};
						for (const [k, v] of Object.entries(next)) {
							numericPorts[Number(k)] = v;
						}
						updateLocal((prev) => (prev ? { ...prev, ports: numericPorts } : undefined));
					}}
					onValidityChange={setPortsValid}
				/>

				<ConfigMount
					disabled={() => !deploymentPermissions().edit}
					value={() => deploymentQuery.data?.configMounts ?? {}}
					onChange={(next) => updateLocal((prev) => (prev ? { ...prev, configMounts: next } : undefined))}
				/>
			</div>

			<Show when={deploymentPermissions().edit}>
				<div class="w-full flex justify-end items-center">
					<Button
						disabled={!deploymentPermissions().edit || isUpdating() || !envValid() || !portsValid()}
						loading={isUpdating()}
						loadingContent={() => <span>Updating...</span>}
						type="submit"
						variant="contained"
					>
						Update
					</Button>
				</div>
			</Show>
		</form>
	);
};

export default DeploymentInfoUpdate;
