import { createFileRoute } from "@tanstack/solid-router";
import { useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createMemo, createResource, createSignal, ErrorBoundary, Match, Show, Suspense, Switch } from "solid-js";
import { GetDeploymentInfoResponse } from "~/bindings";
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
import { createAuthenticatedAction, useAuthState } from "~/hooks";
import useIsAllowed, { useGetPermissions } from "~/hooks/is-allowed";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import DeploymentInfoUpdate from "./-components/info";
import DeploymentLogs from "./-components/logs";
import { Color } from "~/utils/color";

const DeploymentInfo = () => {
	const params = Route.useParams();
	const search = Route.useSearch();
	const tab = () => search().tab;

	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const navigate = useNavigate();
	const [isDeleteModalOpen, setIsDeleteModalOpen] = createSignal(false);

	const resourceParamsDeployment = createMemo(() => {
		return [authState(), workspaceId(), params().id] as const;
	});

	const isAllowedResource = useIsAllowed("deployment", "view", params().id);
	const deploymentPermissions = useGetPermissions("deployment", () => params().id || "");

	const [deploymentInfo, { refetch: refetchDeploymentInfo, mutate: mutateDeploymentInfo }] = createResource(
		resourceParamsDeployment,
		async ([auth, wsId, id]) => {
			if (!wsId || !auth || auth.type !== "LoggedIn" || id === "") {
				return undefined;
			}
			const response = await httpRequest<GetDeploymentInfoResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/deployment/${id}`,
				{
					method: "GET",
				}
			);
			if (!response.ok) {
				console.error("Failed to fetch deployment info:", response.data.error);
				toast("Failed to fetch deployment info", "error");
				return undefined;
			}

			return response.data;
		}
	);

	const { execute: startDeployment, isLoading: isStartingDeployment } = createAuthenticatedAction(
		async ({ workspaceId }) => {
			if (!deploymentPermissions().start) {
				toast("You do not have permission to start this deployment", "error");
				return;
			}

			const deployment = deploymentInfo();
			if (!deployment) {
				toast("Deployment information is not available", "error");
				return;
			}

			const response = await httpRequest(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/deployment/${deployment.id}/start`,
				{
					method: "POST",
				}
			);

			if (!response.ok) {
				console.error("Failed to start deployment:", response.data.error);
				toast("Failed to start deployment", "error");
				return;
			}

			toast("Deployment started successfully", "success");
			refetchDeploymentInfo();
		}
	);

	const { execute: stopDeployment, isLoading: isStoppingDeployment } = createAuthenticatedAction(
		async ({ workspaceId }) => {
			if (!deploymentPermissions().stop) {
				toast("You do not have permission to stop this deployment", "error");
				return;
			}

			const deployment = deploymentInfo();
			if (!deployment) {
				toast("Deployment information is not available", "error");
				return;
			}

			const response = await httpRequest(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/deployment/${deployment.id}/stop`,
				{
					method: "POST",
				}
			);

			if (!response.ok) {
				console.error("Failed to stop deployment:", response.data.error);
				toast("Failed to stop deployment", "error");
				return;
			}

			toast("Deployment stopped successfully", "success");
			refetchDeploymentInfo();
		}
	);

	const { execute: deleteDeployment, isLoading: isDeletingDeployment } = createAuthenticatedAction(
		async ({ workspaceId }) => {
			if (!deploymentPermissions().delete) {
				toast("You do not have permission to delete this deployment", "error");
				return;
			}

			const deployment = deploymentInfo();
			if (!deployment) {
				toast("Deployment information is not available", "error");
				return;
			}

			const resp = await httpRequest(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/deployment/${deployment.id}`,
				{
					method: "DELETE",
				}
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

	const Cta = () => (
		<Switch fallback={<span>Unknown</span>}>
			<Match when={deploymentInfo()?.status === "running"}>
				<Show when={deploymentPermissions().stop}>
					<Button
						onClick={(e) => {
							e.preventDefault();
							stopDeployment();
						}}
						class="h-10"
						variant={ButtonVariant.Outlined}
						color={Color.Error}
						loading={isStoppingDeployment()}
						loadingContent={() => <span>Stopping...</span>}
					>
						Stop
					</Button>
				</Show>
			</Match>
			<Match when={deploymentInfo()?.status === "deploying"}>
				<span class="text-white">Deploying...</span>
			</Match>
			<Match when={deploymentInfo()?.status === "errored"}>
				<span class="text-white">Error occurred</span>
			</Match>
			<Match when={deploymentInfo()?.status === "unreachable"}>
				<span class="text-white">Unreachable</span>
			</Match>
			<Match when={deploymentInfo()?.status === "stopped"}>
				<Show when={deploymentPermissions().start}>
					<Button
						class="h-10"
						variant={ButtonVariant.Contained}
						loading={isStartingDeployment()}
						loadingContent={() => <span>Starting...</span>}
						onClick={(e) => {
							e.preventDefault();
							startDeployment();
						}}
					>
						Start
					</Button>
				</Show>
			</Match>
		</Switch>
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
						<Suspense
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
										label: deploymentInfo() ? deploymentInfo()!.name : "Loading...",
									},
								]}
								subText="A deployment represents a containerized application running on a runner."
								class="justify-between items-center"
								actions={() => (
									<div class="flex items-center justify-end gap-3">
										<Show when={deploymentInfo()?.status}>
											<StatusChip status={deploymentInfo()!.status} />
										</Show>
										{Cta()}

										{deploymentInfo() &&
											deploymentPermissions().delete &&
											deploymentInfo()?.name &&
											deploymentInfo()!.status === "stopped" && (
												<DeleteModal
													isLoading={isDeletingDeployment()}
													title="Do You Really Want to Delete This Deployment?"
													resourceName={deploymentInfo()?.name || ""}
													isOpen={isDeleteModalOpen}
													setIsOpen={setIsDeleteModalOpen}
													onClickDelete={(e) => {
														e.preventDefault();
														deleteDeployment();
													}}
												/>
											)}
									</div>
								)}
								bottomContent={() => (
									<HeadTab
										tab={tab}
										tabItems={[
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
									<Match when={tab() === "logs"}>
										<Show when={deploymentInfo.latest?.id}>
											{(id) => <DeploymentLogs deploymentId={id()} />}
										</Show>
									</Match>
									<Match when={tab() === "info" || tab() === ""}>
										<DeploymentInfoUpdate
											deploymentInfo={deploymentInfo}
											refetchDeploymentInfo={refetchDeploymentInfo}
											mutateDeploymentInfo={mutateDeploymentInfo}
										/>
									</Match>
								</Switch>
							</PageContainerBody>
						</Suspense>
					</ErrorBoundary>
				</PageContainer>
			</Show>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/deployments/$id")({
	validateSearch: (search: Record<string, unknown>): { tab: string } => ({
		tab: (search.tab as string) || "info",
	}),
	component: DeploymentInfo,
});
