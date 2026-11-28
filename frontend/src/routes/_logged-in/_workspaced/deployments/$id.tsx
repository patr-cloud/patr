import { createFileRoute } from "@tanstack/solid-router";
import { useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createSignal, ErrorBoundary, Match, Show, Switch } from "solid-js";
import { FiPause, FiPlay } from "solid-icons/fi";
import {
	Button,
	ButtonVariant,
	DeleteModal,
	HeadTab,
	NoPermissionsPage,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	StatusChip,
	useToast,
	LoadingSpinner,
} from "~/components";
import { createAuthenticatedAction } from "~/hooks";
import useIsAllowed, { useGetPermissions } from "~/hooks/is-allowed";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { useDeploymentInfoQuery, useRunnerInfoQuery } from "~/hooks/fetch";
import { deploymentKeys } from "~/hooks/query-keys";
import { useQueryClient } from "@tanstack/solid-query";
import { httpRequest } from "~/utils/http-request";
import DeploymentEnvironment from "./-components/environment";
import DeploymentInfoUpdate from "./-components/info";
import DeploymentLogs from "./-components/logs";
import DeploymentMetrics from "./-components/metrics";
import { Color } from "~/utils/color";
import { requireUuidParam } from "~/utils/validation";

const DeploymentInfo = () => {
	const params = Route.useParams();
	const search = Route.useSearch();
	const tab = () => search().tab;

	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const navigate = useNavigate();
	const queryClient = useQueryClient();
	const [isDeleteModalOpen, setIsDeleteModalOpen] = createSignal(false);

	const isAllowedResource = useIsAllowed("deployment", "view", params().id);
	const deploymentPermissions = useGetPermissions("deployment", () => params().id || "");

	const deploymentQuery = useDeploymentInfoQuery(() => params().id);

	const deploymentData = () => deploymentQuery.data;

	// For the start/stop toasts. Only fetched with permission to view the runner,
	// so members without it don't get an error toast.
	const runnerPermissions = useGetPermissions("runner", () => deploymentData()?.runner ?? "");
	const runnerQuery = useRunnerInfoQuery(() => (runnerPermissions().view ? (deploymentData()?.runner ?? "") : ""));
	const runner = () => (runnerQuery.isSuccess ? runnerQuery.data?.runner : undefined);

	const refetchDeployment = () => {
		const wsId = workspaceId();
		if (wsId) {
			queryClient.invalidateQueries({ queryKey: deploymentKeys.detail(wsId, params().id) });
		}
	};

	const { execute: startDeployment, isLoading: isStartingDeployment } = createAuthenticatedAction(
		async ({ workspaceId }) => {
			if (!deploymentPermissions().start) {
				toast("You do not have permission to start this deployment", "error");
				return;
			}

			const deployment = deploymentData();
			if (!deployment) {
				toast("Deployment information is not available", "error");
				return;
			}

			const response = await httpRequest(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/deployment/${deployment.id}/start`,
				{ method: "POST" }
			);

			if (!response.ok) {
				console.error("Failed to start deployment:", response.data.error);
				toast("Failed to start deployment", "error");
				return;
			}

			const startRunner = runner();
			if (!startRunner) {
				toast("Start requested.", "success");
			} else if (startRunner.connected) {
				toast(`Start requested. The runner "${startRunner.name}" will deploy "${deployment.name}".`, "success");
			} else {
				toast(
					`Start requested. The runner "${startRunner.name}" is offline, so "${deployment.name}" will start once it reconnects.`,
					"warn"
				);
			}
			refetchDeployment();
		}
	);

	const { execute: stopDeployment, isLoading: isStoppingDeployment } = createAuthenticatedAction(
		async ({ workspaceId }) => {
			if (!deploymentPermissions().stop) {
				toast("You do not have permission to stop this deployment", "error");
				return;
			}

			const deployment = deploymentData();
			if (!deployment) {
				toast("Deployment information is not available", "error");
				return;
			}

			const response = await httpRequest(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/deployment/${deployment.id}/stop`,
				{ method: "POST" }
			);

			if (!response.ok) {
				console.error("Failed to stop deployment:", response.data.error);
				toast("Failed to stop deployment", "error");
				return;
			}

			const stopRunner = runner();
			if (!stopRunner) {
				toast("Stop requested.", "success");
			} else if (stopRunner.connected) {
				toast(
					`Stop requested. The runner "${stopRunner.name}" will shut down "${deployment.name}". Its volumes are kept.`,
					"success"
				);
			} else {
				toast(
					`Stop requested. The runner "${stopRunner.name}" is offline, so "${deployment.name}" will stop once it reconnects.`,
					"warn"
				);
			}
			refetchDeployment();
		}
	);

	const { execute: deleteDeployment, isLoading: isDeletingDeployment } = createAuthenticatedAction(
		async ({ workspaceId }) => {
			if (!deploymentPermissions().delete) {
				toast("You do not have permission to delete this deployment", "error");
				return;
			}

			const deployment = deploymentData();
			if (!deployment) {
				toast("Deployment information is not available", "error");
				return;
			}

			const resp = await httpRequest(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/deployment/${deployment.id}`,
				{ method: "DELETE" }
			);
			console.log("Delete deployment response:", resp);
			if (!resp.ok) {
				toast("Failed to delete deployment", "error");
				return;
			}

			toast("Deployment deleted successfully", "success");
			navigate({ to: "/deployments" });
		}
	);

	return (
		<>
			<Title>Deployment Details | Patr</Title>
			<Show
				when={isAllowedResource()}
				fallback={
					<NoPermissionsPage
						title="Can't View Resource"
						message="You do not have permission to view this deployment."
					/>
				}
			>
				<PageContainer>
					<ErrorBoundary
						fallback={(err, reset) => (
							<div class="flex flex-col items-center justify-center gap-4 py-16">
								<p class="text-error text-sm">Error loading deployment: {err.message}</p>
								<Button variant={ButtonVariant.Outlined} onClick={reset}>
									Retry
								</Button>
							</div>
						)}
					>
						<Show
							when={!deploymentQuery.isPending}
							fallback={
								<div class="flex items-center justify-center gap-2 py-16 text-grey">
									<LoadingSpinner size={20} />
									<span class="text-sm">Loading deployment...</span>
								</div>
							}
						>
							<PageContainerHead
								breadcrumbs={[
									{
										label: "Deployments",
										url: "/deployments",
									},
									{
										label: deploymentData()?.name ?? "Loading...",
									},
								]}
								subText="A deployment represents a containerized application running on a runner."
								class="justify-between items-center"
								actions={() => (
									<div class="flex items-center justify-end gap-4">
										<Show when={deploymentData()?.status}>
											<StatusChip
												status={deploymentData()!.status}
												size="md"
												resourceType="deployment"
											/>
										</Show>
										<Show when={deploymentData()?.shouldRun && deploymentPermissions().stop}>
											<Button
												onClick={(e) => {
													e.preventDefault();
													stopDeployment();
												}}
												aria-label="Stop deployment"
												class="w-10 h-10"
												variant={ButtonVariant.Outlined}
												color={Color.Error}
												loading={isStoppingDeployment()}
												loadingContent={() => <></>}
											>
												<FiPause size={14} />
											</Button>
										</Show>
										<Show
											when={
												deploymentData()?.shouldRun === false && deploymentPermissions().start
											}
										>
											<Button
												aria-label="Start deployment"
												class="w-10 h-10"
												variant={ButtonVariant.Contained}
												loading={isStartingDeployment()}
												loadingContent={() => <></>}
												onClick={(e) => {
													e.preventDefault();
													startDeployment();
												}}
											>
												<FiPlay size={14} />
											</Button>
										</Show>

										{deploymentData() &&
											deploymentPermissions().delete &&
											deploymentData()?.name && (
												<DeleteModal
													isLoading={isDeletingDeployment()}
													title="Do You Really Want to Delete This Deployment?"
													message={
														Object.keys(deploymentData()?.volumes ?? {}).length > 0
															? "Its volumes and the data in them will be deleted too. This action cannot be undone."
															: undefined
													}
													resourceName={deploymentData()?.name || ""}
													isOpen={isDeleteModalOpen}
													setIsOpen={setIsDeleteModalOpen}
													onClickDelete={(e) => {
														e.preventDefault();
														deleteDeployment();
													}}
													renderTrigger={(open) => (
														<Button
															class="h-10"
															onClick={() => open?.(true)}
															variant={ButtonVariant.Outlined}
															color={Color.Error}
														>
															Delete
														</Button>
													)}
												/>
											)}
									</div>
								)}
								bottomContent={() => (
									<HeadTab
										tab={tab}
										tabItems={[
											{
												label: "Metrics",
												value: "metrics",
												onClick: (value) =>
													navigate({
														to: "/deployments/$id",
														params: { id: params().id },
														search: { tab: value },
													}),
											},
											{
												label: "Info",
												value: "info",
												onClick: (value) =>
													navigate({
														to: "/deployments/$id",
														params: { id: params().id },
														search: { tab: value },
													}),
											},
											{
												label: "Configuration",
												value: "environment",
												onClick: (value) =>
													navigate({
														to: "/deployments/$id",
														params: { id: params().id },
														search: { tab: value },
													}),
											},
											{
												label: "Logs",
												value: "logs",
												onClick: (value) =>
													navigate({
														to: "/deployments/$id",
														params: { id: params().id },
														search: { tab: value },
													}),
											},
										]}
									/>
								)}
							/>

							<PageContainerBody class="flex flex-col justify-between gap-8">
								<Switch fallback={<div class="text-grey text-sm py-8 text-center">No such tab</div>}>
									<Match when={tab() === "metrics"}>
										<Show when={deploymentData()?.id}>
											{(id) => <DeploymentMetrics deploymentId={id()} />}
										</Show>
									</Match>
									<Match when={tab() === "info"}>
										<DeploymentInfoUpdate deploymentId={params().id} />
									</Match>
									<Match when={tab() === "environment"}>
										<DeploymentEnvironment deploymentId={params().id} />
									</Match>
									<Match when={tab() === "logs"}>
										<Show when={deploymentData()?.id}>
											{(id) => <DeploymentLogs deploymentId={id()} />}
										</Show>
									</Match>
								</Switch>
							</PageContainerBody>
						</Show>
					</ErrorBoundary>
				</PageContainer>
			</Show>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/deployments/$id")({
	beforeLoad: ({ params }) => requireUuidParam(params.id),
	validateSearch: (search: Record<string, unknown>): { tab: string } => ({
		tab: (search.tab as string) || "metrics",
	}),
	component: DeploymentInfo,
});
