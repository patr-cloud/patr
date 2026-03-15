import { createFileRoute } from "@tanstack/solid-router";
import { useNavigate } from "@tanstack/solid-router";
import { createMemo, createResource, createSignal, ErrorBoundary, Show, Suspense } from "solid-js";
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
	useToast,
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
		async ({ accessToken, workspaceId }) => {
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
		async ({ accessToken, workspaceId }) => {
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
		async ({ accessToken, workspaceId }) => {
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

	const Cta = () => {
		switch (deploymentInfo()?.status) {
			case "running":
				if (!deploymentPermissions().stop) {
					return null;
				}

				return (
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
						STOP
					</Button>
				);

			case "deploying":
				return <span class="text-white">Deploying...</span>;
			case "errored":
				return <span class="text-white">Error occurred</span>;
			case "unreachable":
				return <span class="text-white">Unreachable</span>;
			case "stopped":
				if (!deploymentPermissions().start) {
					return null;
				}

				return (
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
						START
					</Button>
				);
			default:
				return <span>where status?</span>;
		}
	};

	const renderTab = () => {
		switch (tab()) {
			case "logs":
				return <DeploymentLogs deploymentId={deploymentInfo.latest?.id || ""} />;
			case "info":
			case "":
				return (
					<DeploymentInfoUpdate
						deploymentInfo={deploymentInfo}
						refetchDeploymentInfo={refetchDeploymentInfo}
						mutateDeploymentInfo={mutateDeploymentInfo}
					/>
				);
			default:
				return <div class="text-white">No such tab</div>;
		}
	};

	return (
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
						<div>
							<p>Error loading deployment info: {err.message}</p>
							<button onClick={reset}>Retry</button>
						</div>
					)}
				>
					<Suspense fallback={<div>Loading deployment info...</div>}>
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
											value: "",
											onClick: (value) =>
												navigate({ to: "/deployments/$id", params: { id: params().id }, search: { tab: value } }),
										},
										{
											label: "Logs",
											value: "logs",
											onClick: (value) =>
												navigate({ to: "/deployments/$id", params: { id: params().id }, search: { tab: value } }),
										},
									]}
								/>
							)}
						/>

						<PageContainerBody class="flex flex-col justify-between gap-8">{renderTab()}</PageContainerBody>
					</Suspense>
				</ErrorBoundary>
			</PageContainer>
		</Show>
	);
};

export const Route = createFileRoute("/_app/_workspaced/deployments/$id")({
	validateSearch: (search: Record<string, unknown>): { tab: string } => ({
		tab: (search.tab as string) || "",
	}),
	component: DeploymentInfo,
});
