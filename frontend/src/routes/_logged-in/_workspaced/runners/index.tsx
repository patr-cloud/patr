import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Show } from "solid-js";
import { createMemo, createResource, ErrorBoundary, Suspense } from "solid-js";
import { ListRunnersForWorkspaceResponse } from "~/bindings";
import {
	ButtonVariant,
	CopyableField,
	CopyableFieldVariant,
	EmptyState,
	Link,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	Pagination,
	Table,
} from "~/components";
import { useToast } from "~/components";
import { useAuthState, createPaginationState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { useIsAllowed } from "~/hooks";
import { formatRelativeTime } from "~/utils/func";
import { httpRequest } from "~/utils/http-request";

const ListRunnersPage = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const isAllowedCreate = useIsAllowed("runner", "create");
	const navigate = useNavigate();
	const search = Route.useSearch();
	const pagination = createPaginationState({
		search: () => search(),
		navigate,
	})

	const fetchParams = createMemo(() => {
		return [authState(), workspaceId(), pagination.page(), pagination.count()] as const;
	})

	const [runners] = createResource(fetchParams, async ([auth, wsId, page, count]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn") {
			return { runners: [] };
		}
		const response = await httpRequest<ListRunnersForWorkspaceResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/runner?page=${page}&count=${count}`,
			{
				method: "GET",
			}
		)

		if (!response.ok) {
			console.error("Failed to fetch runners:", response.data.error);
			toast("Failed to fetch runners", "error");

			return { runners: [] };
		}

		pagination.setTotalCount(Number(response.headers.get("x-total-count") ?? 0));

		console.log("Fetched runners:", response.data);
		return response.data;
	})

	return (
		<PageContainer>
			<PageContainerHead
				breadcrumbs={[
					{
						label: "Runners",
					},
				]}
				subText="Runners execute deployments on your machines or clusters"
				actions={() => {
					if (!isAllowedCreate()) return null;
					return (
						<Link href="/runners/new" buttonVariant={ButtonVariant.Plain} external={false}>
							Add Runner
						</Link>
					)
				}}
			/>
			<PageContainerBody class="flex flex-col justify-between gap-8">
				<ErrorBoundary
					fallback={(err, reset) => (
						<div>
							<p>Error loading runners: {err.message}</p>
							<button onClick={reset}>Retry</button>
						</div>
					)}
				>
					<Suspense fallback={<div>Loading...</div>}>
						<Show
							when={(runners()?.runners?.length ?? 0) > 0}
							fallback={<EmptyState title="No Runner Added" />}
						>
							<Table
								column_grids={["flex-4", "flex-4", "flex-4", "flex-4"]}
								rows={runners()?.runners || []}
								headings={["ID", "Runner Name", "Status", "Last Seen"]}
								renderRow={(item) => (
									<tr class="border border-border-color min-h-10 flex items-center justify-center w-full px-xl bg-secondary-light last-of-type:rounded-b-xs">
										<td class="flex items-center justify-center min-w-0 flex-1">
											<CopyableField
												variant={CopyableFieldVariant.Text}
												value={item.id}
												class="truncate"
												innerClass="text-white"
												buttonPosition="start"
											/>
										</td>
										<td class="flex items-center justify-center min-w-0 flex-1">{item.name}</td>
										<td class="flex items-center justify-center min-w-0 flex-1">
											{item.connected ? "Connected" : "Disconnected"}
										</td>
										<td class="flex items-center justify-center min-w-0 flex-1">
											{item.lastSeen ? formatRelativeTime(item.lastSeen) : "N/A"}
										</td>
									</tr>
								)}
							/>
							<Pagination
								state={pagination}
								loading={runners.loading}
								showPageSizeSelector={false}
								showGoToPage={false}
							/>
						</Show>
					</Suspense>
				</ErrorBoundary>
			</PageContainerBody>
		</PageContainer>
	)
};

export const Route = createFileRoute("/_logged-in/_workspaced/runners/")({
	validateSearch: (search: Record<string, unknown>): { page?: string; count?: string } => ({
		page: (search.page as string) || undefined,
		count: (search.count as string) || undefined,
	}),
	component: ListRunnersPage,
});
