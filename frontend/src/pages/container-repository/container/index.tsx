import { useNavigate, useParams, useSearchParams } from "@solidjs/router";
import { createMemo, createResource, createSignal, ErrorBoundary, Suspense } from "solid-js";
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
import { GetContainerRepositoryInfoResponse, ListContainerRepositoryTagsResponse } from "~/bindings";
import { httpRequest } from "~/utils/http-request";
import General from "./general";
import Images from "./images";

const ContainerInfo = () => {
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
		console.log("Fetching repository info for repoId:", wsId, repoId);
		const response = await httpRequest<GetContainerRepositoryInfoResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry/${repoId}`,
			{
				method: "GET",
				headers: {
					"Content-Type": "application/json",
					Authorization: `Bearer ${auth.accessToken}`,
				},
			}
		);
		console.log("Fetched repository info:", response);
		if (!response.ok) {
			console.error("Failed to fetch repository info:", response.data.error);
			toast("Failed to fetch repository info", "error");
			return undefined;
		}

		return response.data;
	});

	const [imageTags] = createResource(resourceParams, async ([auth, wsId, repoId]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn" || !repoId) {
			return undefined;
		}
		console.log("Fetching image tags for repository:", repoId);
		const response = await httpRequest<ListContainerRepositoryTagsResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry/${repoId}/tag`,
			{
				method: "GET",
				headers: {
					"Content-Type": "application/json",
					Authorization: `Bearer ${auth.accessToken}`,
				},
			}
		);
		console.log("Fetched image tags:", response);
		if (!response.ok) {
			console.error("Failed to fetch image tags:", response.data.error);
			toast("Failed to fetch image tags", "error");
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
			console.error("User not logged in or workspace ID missing");
			return;
		}

		const resp = await httpRequest(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId()}/docker-registry/${params.id}`,
			{
				method: "DELETE",
				headers: {
					"Content-Type": "application/json",
					Authorization: `Bearer ${auth.accessToken}`,
				},
			}
		);
		console.log("Delete container-repository response:", resp);
		if (!resp.ok) {
			toast("Failed to delete repository", "error");
			return;
		}

		toast("Repository deleted successfully", "success");
		navigate("/container-repositories");
	};

	const renderTab = () => {
		switch (tab()) {
			case "images":
				const tags = imageTags();
				if (!tags) return <div>Loading image tags...</div>;
				return <Images imageTags={tags} />;
			case "general":
			case "":
			default:
				return <General repositoryInfo={repositoryInfo()} />;
		}
	};

	return (
		<PageContainer>
			<PageContainerHead
				breadcrumbs={[
					{
						label: "Repository",
						url: "/container-repositories",
					},
					{
						label: repositoryInfo()?.repository?.name || "Loading...",
					},
				]}
				subText="View and manage container repository images"
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
export default ContainerInfo;
