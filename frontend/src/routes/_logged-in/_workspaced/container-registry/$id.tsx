import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createMemo, createResource, createSignal, ErrorBoundary, Match, Show, Suspense, Switch } from "solid-js";
import {
	Button,
	ButtonVariant,
	DeleteModal,
	HeadTab,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	useToast,
	LoadingSpinner,
} from "~/components";
import { useAuthState, useIsAllowed } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { GetContainerRepositoryInfoResponse, ListContainerRepositoryManifestsResponse } from "~/bindings";
import { httpRequest } from "~/utils/http-request";
import General from "./-components/general";
import Images from "./-components/images";

const ContainerRepositoryInfo = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const isAllowedDelete = useIsAllowed("containerRegistryRepository", "delete", undefined);
	const toast = useToast();
	const navigate = useNavigate();
	const params = Route.useParams();
	const search = Route.useSearch();
	const [isDeleteModalOpen, setIsDeleteModalOpen] = createSignal(false);
	const tab = () => search().tab;

	const resourceParams = createMemo(() => {
		return [authState(), workspaceId(), params().id] as const;
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
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId()}/container-registry/${params().id}`,
			{
				method: "DELETE",
			}
		);
		if (!resp.ok) {
			toast("Failed to delete repository", "error");
			return;
		}

		toast("Repository deleted successfully", "success");
		navigate({ to: "/container-registry" });
	};

	return (
		<>
			<Title>Repository Details | Patr</Title>
			<PageContainer>
				<ErrorBoundary
					fallback={(err, reset) => (
						<div class="flex flex-col items-center justify-center gap-4 py-16">
							<p class="text-error text-sm">Error loading repository: {err.message}</p>
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
								<span class="text-sm">Loading repository...</span>
							</div>
						}
					>
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
									<Show when={isAllowedDelete() && repositoryInfo()?.repository?.name}>
										<DeleteModal
											title="Do You Really Want to Delete This Repository?"
											resourceName={repositoryInfo()?.repository?.name || ""}
											onClickDelete={handleDelete}
											isOpen={isDeleteModalOpen}
											setIsOpen={setIsDeleteModalOpen}
										/>
									</Show>
								</div>
							)}
							bottomContent={() => (
								<HeadTab
									tab={tab}
									tabItems={[
										{
											label: "General",
											value: "",
											onClick: (value) =>
												navigate({
													to: "/container-registry/$id",
													params: { id: params().id },
													search: { tab: value },
												}),
										},
										{
											label: "Images",
											value: "images",
											onClick: (value) =>
												navigate({
													to: "/container-registry/$id",
													params: { id: params().id },
													search: { tab: value },
												}),
										},
									]}
								/>
							)}
						/>

						<PageContainerBody class="flex flex-col justify-between gap-8">
							<Switch fallback={<General repositoryInfo={() => repositoryInfo()} />}>
								<Match when={tab() === "images"}>
									<Show
										when={manifests()}
										fallback={
											<div class="flex items-center justify-center gap-2 py-16 text-grey">
												<LoadingSpinner size={20} />
												<span class="text-sm">Loading images...</span>
											</div>
										}
									>
										{(manifestList) => (
											<Images manifests={manifestList} refetch={refetchManifests} />
										)}
									</Show>
								</Match>
							</Switch>
						</PageContainerBody>
					</Suspense>
				</ErrorBoundary>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/container-registry/$id")({
	validateSearch: (search: Record<string, unknown>): { tab: string } => ({
		tab: (search.tab as string) || "",
	}),
	component: ContainerRepositoryInfo,
});
