import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createSignal, ErrorBoundary, Match, Show, Switch } from "solid-js";
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
import { useContainerRegistryInfoQuery, useContainerManifestsQuery } from "~/hooks/fetch";
import { containerRegistryKeys } from "~/hooks/query-keys";
import { useQueryClient } from "@tanstack/solid-query";
import { httpRequest } from "~/utils/http-request";
import General from "./-components/general";
import Versions from "./-components/versions";

const ContainerRepositoryInfo = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const isAllowedDelete = useIsAllowed("containerRegistryRepository", "delete", undefined);
	const toast = useToast();
	const navigate = useNavigate();
	const queryClient = useQueryClient();
	const params = Route.useParams();
	const search = Route.useSearch();
	const [isDeleteModalOpen, setIsDeleteModalOpen] = createSignal(false);
	const tab = () => search().tab;

	const repoInfoQuery = useContainerRegistryInfoQuery(() => params().id);
	const manifestsQuery = useContainerManifestsQuery(() => params().id);

	const refetchManifests = () => {
		const wsId = workspaceId();
		if (wsId) {
			queryClient.invalidateQueries({ queryKey: containerRegistryKeys.manifests(wsId, params().id) });
		}
	};

	const handleDelete = async (
		e: MouseEvent & {
			currentTarget: HTMLButtonElement;
		}
	) => {
		e.preventDefault();

		const auth = authState();
		const currentWorkspace = workspaceId();
		const repository = repoInfoQuery.data;

		if (!auth || auth.type !== "LoggedIn" || !currentWorkspace || !repository) {
			return;
		}

		const resp = await httpRequest(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId()}/container-registry/${params().id}`,
			{ method: "DELETE" }
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
					<Show
						when={!repoInfoQuery.isPending}
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
									label: repoInfoQuery.data?.repository?.name || "Loading...",
								},
							]}
							subText="Store and manage container images for your deployments"
							class="justify-between items-center"
							actions={() => (
								<div class="flex items-center justify-end gap-3">
									<Show when={isAllowedDelete() && repoInfoQuery.data?.repository?.name}>
										<DeleteModal
											title="Do You Really Want to Delete This Repository?"
											resourceName={repoInfoQuery.data?.repository?.name || ""}
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
											label: "Overview",
											value: "",
											onClick: (value) =>
												navigate({
													to: "/container-registry/$id",
													params: { id: params().id },
													search: { tab: value },
												}),
										},
										{
											label: "Tags",
											value: "tags",
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
							<Switch fallback={<General repositoryInfo={() => repoInfoQuery.data} />}>
								<Match when={tab() === "tags"}>
									<Show
										when={manifestsQuery.data}
										fallback={
											<div class="flex items-center justify-center gap-2 py-16 text-grey">
												<LoadingSpinner size={20} />
												<span class="text-sm">Loading images...</span>
											</div>
										}
									>
										{(manifestList) => (
											<Versions
												repoId={params().id}
												imagePath={`registry.patr.cloud/${workspaceId() ?? ""}/${
													repoInfoQuery.data?.repository?.name ?? ""
												}`}
												manifests={manifestList}
												refetch={refetchManifests}
											/>
										)}
									</Show>
								</Match>
							</Switch>
						</PageContainerBody>
					</Show>
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
