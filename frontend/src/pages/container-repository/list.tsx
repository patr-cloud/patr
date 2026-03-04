import { createMemo, createResource, ErrorBoundary, Show, Suspense } from "solid-js";
import { useNavigate } from "@solidjs/router";
import {
	ButtonVariant,
	EmptyState,
	Link,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	Table,
	useToast,
} from "~/components";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { ListContainerRepositoriesResponse, WithId, ContainerRepository } from "~/bindings";
import { httpRequest } from "~/utils/http-request";
import { formatRelativeTime, formatSize } from "~/utils/func";

const ListContainerRepositories = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const navigate = useNavigate();

	const resourceParams = createMemo(() => {
		return [authState(), workspaceId()] as const;
	});

	const [repositories] = createResource(resourceParams, async ([auth, wsId]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn") {
			return undefined;
		}

		const response = await httpRequest<ListContainerRepositoriesResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry`,
			{
				method: "GET",
				headers: {
					"Content-Type": "application/json",
					Authorization: `Bearer ${auth.accessToken}`,
				},
			}
		);

		if (!response.ok) {
			toast("Failed to fetch repositories", "error");
			return undefined;
		}

		return response.data;
	});

	return (
		<PageContainer>
			<PageContainerHead
				breadcrumbs={[
					{
						label: "Container Repositories",
					},
				]}
				subText="Store and manage container images for your deployments"
				actions={() => (
					<Link href="/container-registry/new" buttonVariant={ButtonVariant.Plain} external={false}>
						Create Repository
					</Link>
				)}
			/>

			<PageContainerBody class="flex flex-col">
				<ErrorBoundary
					fallback={(err, reset) => (
						<div>
							<p>Error loading repositories: {err.message}</p>
							<button onClick={reset}>Retry</button>
						</div>
					)}
				>
					<Suspense fallback={<div>Loading repositories...</div>}>
						<Show
							when={repositories()?.repositories && repositories()!.repositories.length > 0}
							fallback={<EmptyState title="No Container Repositories Exist" />}
						>
							<Table
								column_grids={["flex-1", "flex-1", "flex-1", "flex-1"]}
								headings={["Container Repository", "Last Updated", "Size", "Created At"]}
								rows={repositories()?.repositories || []}
								renderRow={(repo: WithId<ContainerRepository>) => (
									<tr class="table-row cursor-pointer" onClick={() => navigate(`/container-registry/${repo.id}`)}>
										<td class="flex-1">
											<span class="truncate">{repo.name}</span>
										</td>
										<td class="flex-1">{formatRelativeTime(repo.lastUpdated)}</td>
										<td class="flex-1">{formatSize(repo.size)}</td>
										<td class="flex-1">{formatRelativeTime(repo.created)}</td>
									</tr>
								)}
							/>
						</Show>
					</Suspense>
				</ErrorBoundary>
			</PageContainerBody>
		</PageContainer>
	);
};

export default ListContainerRepositories;
