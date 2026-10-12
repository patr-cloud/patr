import { FiChevronDown } from "solid-icons/fi";
import { createEffect, createMemo, createSignal, Show } from "solid-js";
import { ExposedPortType, GetDeploymentInfoResponse, UpdateDeploymentResponse } from "~/bindings";
import {
	Button,
	CopyableField,
	CopyableFieldVariant,
	Input,
	InputType,
	InputDropdown,
	Label,
	RangeSlider,
	ToggleSwitch,
	UnsavedChangesGuard,
	useToast,
} from "~/components";
import { useAuthState } from "~/hooks";
import { useGetPermissions } from "~/hooks/is-allowed";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { useDeploymentInfoQuery, useRunnerInfoQuery, useRunnersInfiniteQuery } from "~/hooks/fetch";
import { deploymentKeys } from "~/hooks/query-keys";
import { useQueryClient } from "@tanstack/solid-query";
import { REGISTRY_DOMAIN } from "~/utils/env";
import { httpRequest } from "~/utils/http-request";
import { EventT } from "~/utils/types";
import PortInput from "./port";
import { isSameUpdate, toUpdateRequest } from "./utils";

interface DeploymentInfoProps {
	deploymentId: string;
}

const DeploymentInfoUpdate = (props: DeploymentInfoProps) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const queryClient = useQueryClient();

	const deploymentQuery = useDeploymentInfoQuery(() => props.deploymentId);
	const [runnerSearch, setRunnerSearch] = createSignal("");
	const runnersQuery = useRunnersInfiniteQuery(runnerSearch);

	// Local signal for form editing — initialized from query data and kept in sync
	const [localInfo, setLocalInfo] = createSignal<GetDeploymentInfoResponse | undefined>(undefined);

	createEffect(() => {
		if (deploymentQuery.data && !localInfo()) {
			setLocalInfo(deploymentQuery.data);
		}
	});

	const loadedRunners = () => runnersQuery.data?.pages.flatMap((page) => page.runners) ?? [];

	// The picker only holds the pages scrolled so far, so the selected runner
	// may not be among them yet. Fetch it on its own so the field isn't blank.
	const missingRunnerId = () => {
		const id = localInfo()?.runner;
		return id && !loadedRunners().some((runner) => runner.id === id) ? id : "";
	};
	const missingRunnerQuery = useRunnerInfoQuery(missingRunnerId);

	const runnerOptions = () => {
		const options = loadedRunners().map((runner) => ({ value: runner.id, label: runner.name }));
		// Read through `isSuccess` so a pending fetch doesn't suspend the form.
		const missing = missingRunnerQuery.isSuccess ? missingRunnerQuery.data?.runner : undefined;
		if (missing && missingRunnerId() === missing.id) {
			options.unshift({ value: missing.id, label: missing.name });
		}
		return options;
	};

	const deploymentPermissions = useGetPermissions("deployment", () => props.deploymentId);

	const [_, setHasUpdated] = createSignal(false);
	const [isUpdating, setIsUpdating] = createSignal(false);
	const [portsValid, setPortsValid] = createSignal(true);

	// Whether the draft would change anything if saved. Update stays disabled
	// until it would.
	const isDirty = createMemo(() => {
		const info = localInfo();
		const saved = deploymentQuery.data;
		return !!info && !!saved && !isSameUpdate(info, saved);
	});

	type DeployInfo = GetDeploymentInfoResponse | undefined;
	const updateLocal = (fn: (prev: DeployInfo) => DeployInfo) => {
		setHasUpdated(true);
		setLocalInfo(fn);
	};

	// Volumes are node-local, so a deployment with any runs a single replica.
	const hasVolumes = () => Object.keys(localInfo()?.volumes ?? {}).length > 0;
	createEffect(() => {
		const info = localInfo();
		if (hasVolumes() && info && (info.minHorizontalScale > 1 || info.maxHorizontalScale > 1)) {
			updateLocal((prev) => (prev ? { ...prev, minHorizontalScale: 1, maxHorizontalScale: 1 } : undefined));
		}
	});

	const isPatrRegistry = () => {
		const info = localInfo();
		if (!info) return false;
		return info.registry === REGISTRY_DOMAIN;
	};

	const refetchDeploymentInfo = async () => {
		const wsId = workspaceId();
		if (wsId) {
			await queryClient.invalidateQueries({ queryKey: deploymentKeys.detail(wsId, props.deploymentId) });
		}
	};

	const onSubmitUpdate = async (e: EventT<SubmitEvent, HTMLFormElement>) => {
		e.preventDefault();
		const auth = authState();
		if (!auth || auth.type !== "LoggedIn") {
			toast("User not logged in", "error");
			return;
		}

		// The Update button is disabled when env/ports are invalid, but form
		// submission can still be triggered via Enter on another input or
		// programmatically. Block invalid payloads defensively.
		if (!portsValid()) {
			toast("Please fix the highlighted errors before saving", "error");
			return;
		}

		const info = localInfo();
		if (!info) {
			toast("Deployment info not available", "error");
			return;
		}

		const body = toUpdateRequest(info);

		setIsUpdating(true);
		try {
			const response = await httpRequest<UpdateDeploymentResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId()}/deployment/${info.id}`,
				{
					method: "PATCH",
					body: JSON.stringify(body),
				}
			);

			if (!response.ok) {
				console.error("Failed to update deployment:", response.data.error);
				toast("Failed to update deployment", "error");
				refetchDeploymentInfo();
				return;
			}

			toast("Deployment updated successfully", "success");
			// Re-seed from what the server now holds, so anything it normalised
			// doesn't leave the form looking unsaved.
			await refetchDeploymentInfo();
			setLocalInfo(deploymentQuery.data);
		} finally {
			setIsUpdating(false);
		}
	};

	return (
		<form onSubmit={onSubmitUpdate} class="flex flex-col gap-6 justify-between w-full flex-1">
			<div class="flex flex-col gap-4 items-start w-full">
				<div class="flex gap-8 items-start w-full">
					<Label parentClass="flex-2 pt-2.5" for="deployment-id" label="ID" />
					<CopyableField
						value={localInfo()?.id ?? ""}
						variant={CopyableFieldVariant.Input}
						buttonPosition="start"
						class="flex-10"
					/>
				</div>

				<div class="flex gap-8 items-start w-full">
					<Label parentClass="flex-2 pt-2.5" for="deployment-name" label="Name" />
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
					<Label
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
					<Label parentClass="flex-2 pt-2.5" for="deployment-runner" label="Runner" />

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
						options={runnerOptions()}
						onLoadMore={runnersQuery.hasNextPage ? () => runnersQuery.fetchNextPage() : undefined}
						isLoadingMore={() => runnersQuery.isFetchingNextPage || runnersQuery.isPlaceholderData}
						onSearch={setRunnerSearch}
						onSelect={(runnerId) => {
							updateLocal((prev) => (prev ? { ...prev, runner: runnerId } : undefined));
						}}
					/>
				</div>

				<div class="flex gap-8 items-start w-full">
					<Label parentClass="flex-2 pt-2.5" for="deployment-registry" label="Image" />
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
								if (info.registry === REGISTRY_DOMAIN) {
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
					<Label
						parentClass="flex-2"
						label="Horizontal Scale"
						comments={
							hasVolumes() ? "Deployments with volumes run a single replica" : "Min & max replica count"
						}
					/>
					<div class="flex-10">
						<RangeSlider
							min={1}
							max={10}
							valueLow={() => localInfo()?.minHorizontalScale ?? 1}
							valueHigh={() => localInfo()?.maxHorizontalScale ?? 2}
							disabled={!deploymentPermissions().edit || hasVolumes()}
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
						<Label parentClass="flex-2" label="Deploy on Push" comments="Redeploy on new image push" />
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
			</div>

			<Show when={deploymentPermissions().edit}>
				{/* Sticky, so saving doesn't mean scrolling to the end of a long form. */}
				<div class="sticky bottom-0 z-10 w-full flex justify-end items-center gap-4 py-4 bg-secondary-dark border-t border-border-color">
					<Show when={isDirty()}>
						<span class="text-sm text-grey">Unsaved changes</span>
					</Show>
					<Button
						disabled={!isDirty() || isUpdating() || !portsValid()}
						loading={isUpdating()}
						loadingContent={() => <span>Updating...</span>}
						type="submit"
						variant="contained"
					>
						Update
					</Button>
				</div>
			</Show>

			<UnsavedChangesGuard
				when={isDirty}
				message="You have unsaved changes to this deployment. If you leave now, they'll be lost."
			/>
		</form>
	);
};

export default DeploymentInfoUpdate;
