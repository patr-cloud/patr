import { useNavigate, useParams, useSearchParams } from "@solidjs/router";
import { createMemo, createResource, createSignal, ErrorBoundary, Suspense } from "solid-js";
import { GetDeploymentInfoResponse } from "~/bindings";
import {
	Button,
	ButtonVariant,
	DeleteModal,
	HeadTab,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	useToast,
} from "~/components";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import DeploymentInfoUpdate from "~/pages/deployment/deployment/info";
import DeploymentLogs from "./logs";
import { Color } from "~/utils/color";
import { useResourceIdPermissions } from "~/hooks/use-fetch/use-allowed";

const DeploymentInfo = () => {
	const params = useParams();
	const [searchParams, setSearchParams] = useSearchParams();
	const tab = () => (searchParams.tab as string) || "";
	// if (!params.id) {
	// 	return <div class="text-white">No deployment ID provided in URL</div>;
	// }
	// const [isAllowedView] = useIsAllowed("deployment", "view", () => params.id!);
	const permissions = useResourceIdPermissions("deployment", () => params.id!);
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const navigate = useNavigate();
	const [isDeleteModalOpen, setIsDeleteModalOpen] = createSignal(false);

	const resourceParamsDeployment = createMemo(() => {
		return [authState(), workspaceId(), params.id] as const;
	});

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
					headers: {
						"Content-Type": "application/json",
						Authorization: `Bearer ${auth.accessToken}`,
					},
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

	const onClickStart = async (e: MouseEvent & { currentTarget: HTMLButtonElement }) => {
		e.preventDefault();

		const auth = authState();
		const currentWorkspace = workspaceId();
		const deployment = deploymentInfo();

		if (!auth || auth.type !== "LoggedIn" || !currentWorkspace || !deployment) {
			console.error("User not logged in or workspace ID missing");
			return;
		}

		console.log("Start deployment clicked");
		const resp = await httpRequest(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId()}/deployment/${deployment.id}/start`,
			{
				method: "POST",
				headers: {
					"Content-Type": "application/json",
					Authorization: `Bearer ${auth.accessToken}`,
				},
			}
		);
		console.log("Start deployment response:", resp);
	};

	const onClickStop = async (e: MouseEvent & { currentTarget: HTMLButtonElement }) => {
		e.preventDefault();

		const auth = authState();
		const currentWorkspace = workspaceId();
		const deployment = deploymentInfo();

		if (!auth || auth.type !== "LoggedIn" || !currentWorkspace || !deployment) {
			console.error("User not logged in or workspace ID missing");
			return;
		}

		console.log("Stop deployment clicked");
		const resp = await httpRequest(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId()}/deployment/${deployment.id}/stop`,
			{
				method: "POST",
				headers: {
					"Content-Type": "application/json",
					Authorization: `Bearer ${auth.accessToken}`,
				},
			}
		);
		console.log("Stop deployment response:", resp);
	};

	const onClickDelete = async (
		e: MouseEvent & {
			currentTarget: HTMLButtonElement;
		}
	) => {
		e.preventDefault();

		if (!permissions()?.delete) {
			toast("You do not have permission to delete this deployment", "error");
			return;
		}

		const auth = authState();
		const currentWorkspace = workspaceId();
		const deployment = deploymentInfo();

		if (!auth || auth.type !== "LoggedIn" || !currentWorkspace || !deployment) {
			console.error("User not logged in or workspace ID missing");
			return;
		}

		console.log("Delete deployment clicked");
		const resp = await httpRequest(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId()}/deployment/${deployment.id}`,
			{
				method: "DELETE",
				headers: {
					"Content-Type": "application/json",
					Authorization: `Bearer ${auth.accessToken}`,
				},
			}
		);
		console.log("Delete deployment response:", resp);
		if (!resp.ok) {
			toast("Failed to delete deployment", "error");
			return;
		}

		toast("Deployment deleted successfully", "success");
		navigate("/deployments");
	};

	const Cta = () => {
		switch (deploymentInfo()?.status) {
			case "running":
				if (!permissions()?.edit) {
					return null;
				}

				return (
					<Button onClick={onClickStop} class="h-10" variant={ButtonVariant.Outlined} color={Color.Error}>
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
				if (!permissions()?.edit) {
					return null;
				}

				return (
					<Button onClick={onClickStart} class="h-10" variant={ButtonVariant.Contained}>
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
						hasEditPermission={permissions()?.edit || false}
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
		<PageContainer>
			<PageContainerHead
				title="Deployments"
				titleUrl="/deployments"
				class="justify-between items-center"
				subTitle={
					<Suspense fallback={<div>loading...</div>}>{deploymentInfo()?.name || "No Deployment Found"}</Suspense>
				}
				actions={() => (
					<div class="flex items-center justify-end gap-3">
						<Suspense fallback={<div>Loading actions...</div>}>
							{Cta()}

							{deploymentInfo() && deploymentInfo()?.name && deploymentInfo()!.status === "stopped" && (
								<DeleteModal
									title="Do You Really Want to Delete This Deployment?"
									resourceName={deploymentInfo()?.name || ""}
									onClickDelete={onClickDelete}
									isOpen={isDeleteModalOpen}
									setIsOpen={setIsDeleteModalOpen}
								/>
							)}
						</Suspense>
					</div>
				)}
				bottomContent={() => (
					<HeadTab
						tab={tab}
						searchParams={searchParams}
						setSearchParams={setSearchParams}
						tabItems={[
							{
								label: "Info",
								value: "",
								onClick: (value) => setSearchParams({ tab: value }),
							},
							{
								label: "Logs",
								value: "logs",
								onClick: (value) => setSearchParams({ tab: value }),
							},
						]}
					/>
				)}
			/>
			<PageContainerBody class="flex flex-col justify-between gap-8">
				<ErrorBoundary
					fallback={(err, reset) => (
						<div>
							<p>Error loading deployment info: {err.message}</p>
							<button onClick={reset}>Retry</button>
						</div>
					)}
				>
					<Suspense fallback={<div>Loading deployment info...</div>}>{renderTab()}</Suspense>
				</ErrorBoundary>
			</PageContainerBody>
		</PageContainer>
	);
};

export default DeploymentInfo;
