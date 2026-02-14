import { createMemo, createResource, createSignal, For, Show } from "solid-js";
import { useNavigate } from "@solidjs/router";
import {
	ButtonVariant,
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
import { formatRelativeTime } from "~/utils/func";

const ListContainerRepository = () => {
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
			console.error("Failed to fetch repositories:", response.data.error);
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
						label: "Repository",
					},
				]}
				subText="Create Deployments, Databases, Object Storage, Static Sites, Upgrade Paths and manage Repositories"
				actions={() => (
					<Link href="/container-repositories/new" buttonVariant={ButtonVariant.Plain} external={false}>
						Add Repository
					</Link>
				)}
			/>

			<PageContainerBody>
				<div class="w-full flex flex-col gap-6">
					<div class="w-full">
						<Show
							when={repositories()?.repositories && repositories()!.repositories.length > 0}
							fallback={
								<div class="w-full text-center py-16">
									<p class="text-white text-lg">No Repository Exist</p>
								</div>
							}
						>
							<Table
								column_grids={["flex-1", "flex-1"]}
								headings={["Repository", "Date Created"]}
								rows={repositories()?.repositories || []}
								renderRow={(repo: WithId<ContainerRepository>) => (
									<tr class="table-row" onClick={() => navigate(`/container-repositories/${repo.id}`)}>
										<td class="flex-1">
											<span class="truncate">{repo.name}</span>
										</td>
										<td class="flex-1">{formatRelativeTime(repo.created)}</td>
									</tr>
								)}
							/>
						</Show>
					</div>
				</div>
			</PageContainerBody>
		</PageContainer>
	);
};

export default ListContainerRepository;
