import { useNavigate, useParams, useSearchParams } from "@solidjs/router";
import { createMemo, createResource, createSignal, ErrorBoundary, Suspense } from "solid-js";
import { DeleteModal, HeadTab, PageContainer, PageContainerBody, PageContainerHead, useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { GetContainerRepositoryInfoResponse, ListContainerRepositoryManifestsResponse } from "~/bindings";
import { httpRequest } from "~/utils/http-request";
import General from "./general";
import Images from "./images";

const ContainerRepositoryInfo = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const navigate = useNavigate();
	const params = useParams();

	const [searchParams, setSearchParams] = useSearchParams();
	const [isDeleteModalOpen, setIsDeleteModalOpen] = createSignal(false);
	const tab = () => (searchParams.tab as string) || "";

	const resourceParams = createMemo(() => {
		return [authState(), workspaceId(), params.id] as const;
	});

	const [repositoryInfo] = createResource(resourceParams, async ([auth, wsId, repoId]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn" || !repoId) {
			return undefined;
		}
		const response = await httpRequest<GetContainerRepositoryInfoResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry/${repoId}`,
			{
				method: "GET",
			}
		);
		if (!response.ok) {
			toast("Failed to fetch repository info", "error");
			return undefined;
		}

		return response.data;
	});

	const [manifests, { refetch: refetchManifests }] = createResource(resourceParams, async ([auth, wsId, repoId]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn" || !repoId) {
			return undefined;
		}
		const response = await httpRequest<ListContainerRepositoryManifestsResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry/${repoId}/manifest`,
			{
				method: "GET",
			}
		);
		if (!response.ok) {
			toast("Failed to fetch manifests", "error");
			return undefined;
		}

		return response.data;
	});

	const handleDelete = async (
		e: MouseEvent & {
			currentTarget: HTMLButtonElement;
		}
	) => {
		e.preventDefault();

		const auth = authState();
		const currentWorkspace = workspaceId();
		const repository = repositoryInfo();

		if (!auth || auth.type !== "LoggedIn" || !currentWorkspace || !repository) {
			return;
		}

		const resp = await httpRequest(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId()}/container-registry/${params.id}`,
			{
				method: "DELETE",
			}
		);
		if (!resp.ok) {
			toast("Failed to delete repository", "error");
			return;
		}

		toast("Repository deleted successfully", "success");
		navigate("/container-registry");
	};

	const renderTab = () => {
		switch (tab()) {
			case "images":
				const manifest_list = manifests();
				if (!manifest_list) return <div>Loading manifests...</div>;
				return <Images manifests={() => manifest_list} refetch={refetchManifests} />;
			case "general":
			case "":
			default:
				return <General repositoryInfo={() => repositoryInfo()} />;
		}
	};

	return (
		<PageContainer>
			<PageContainerHead
				breadcrumbs={[
					{
						label: "Container Repositories",
						url: "/container-registry",
					},
					{
						label: repositoryInfo()?.repository?.name || "Loading...",
					},
				]}
				subText="Store and manage container images for your deployments"
				class="justify-between items-center"
				actions={() => (
					<div class="flex items-center justify-end gap-3">
						<Suspense fallback={<div>Loading actions...</div>}>
							{repositoryInfo() && repositoryInfo()?.repository?.name && (
								<DeleteModal
									title="Do You Really Want to Delete This Repository?"
									resourceName={repositoryInfo()?.repository?.name || ""}
									onClickDelete={handleDelete}
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
								label: "General",
								value: "",
								onClick: (value) => setSearchParams({ tab: value }),
							},
							{
								label: "Images",
								value: "images",
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
							<p>Error loading repository info: {err.message}</p>
							<button onClick={reset}>Retry</button>
						</div>
					)}
				>
					<Suspense fallback={<div>Loading repository info...</div>}>{renderTab()}</Suspense>
				</ErrorBoundary>
			</PageContainerBody>
		</PageContainer>
	);
};
export default ContainerRepositoryInfo;
