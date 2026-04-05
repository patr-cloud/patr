import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { Show, createMemo, createResource, ErrorBoundary, Suspense } from "solid-js";
import { ListRunnersForWorkspaceResponse } from "~/bindings";
import {
	Button,
	ButtonVariant,
	CopyableField,
	CopyableFieldVariant,
	EmptyState,
	Link,
	LoadingSpinner,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	Pagination,
	Table,
	StatusChip,
	useToast,
} from "~/components";
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
	});

	const fetchParams = createMemo(() => {
		return [authState(), workspaceId(), pagination.page(), pagination.count()] as const;
	});

	const [runners] = createResource(fetchParams, async ([auth, wsId, page, count]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn") {
			return { runners: [] };
		}
		const response = await httpRequest<ListRunnersForWorkspaceResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/runner?page=${page}&count=${count}`,
			{
				method: "GET",
			}
		);

		if (!response.ok) {
			console.error("Failed to fetch runners:", response.data.error);
			toast("Failed to fetch runners", "error");

			return { runners: [] };
		}

		pagination.setTotalCount(Number(response.headers.get("x-total-count") ?? 0));

		console.log("Fetched runners:", response.data);
		return response.data;
	});

	return (
		<>
			<Title>Runners | Patr</Title>
			<PageContainer>
				<PageContainerHead
					breadcrumbs={[
						{
							label: "Runners",
						},
					]}
					subText="Runners execute deployments on your machines or clusters"
					actions={() => (
						<Show when={isAllowedCreate() && (runners()?.runners?.length ?? 0) > 0}>
							<Link href="/runners/new" buttonVariant={ButtonVariant.Outlined} external={false}>
								Add Runner
							</Link>
						</Show>
					)}
				/>
				<PageContainerBody class="flex flex-col justify-between">
					<ErrorBoundary
						fallback={(err, reset) => (
							<div class="flex flex-col items-center justify-center gap-4 py-16">
								<p class="text-error text-sm">Error loading runners: {err.message}</p>
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
									<span class="text-sm">Loading runners...</span>
								</div>
							}
						>
							<Show
								when={(runners()?.runners?.length ?? 0) > 0}
								fallback={
									<EmptyState
										title="No Runners Added"
										description={
											isAllowedCreate()
												? "Connect your infrastructure to start deploying."
												: undefined
										}
										action={
											isAllowedCreate() ? (
												<Link
													href="/runners/new"
													buttonVariant={ButtonVariant.Outlined}
													external={false}
												>
													Add Runner
												</Link>
											) : undefined
										}
									/>
								}
							>
								<Table
									column_grids={["flex-4", "flex-2", "flex-3", "flex-3"]}
									rows={runners()?.runners || []}
									headings={["Name", "Status", "Last Seen", "ID"]}
									renderRow={(item) => {
										const goToDetail = () =>
											navigate({
												to: "/runners/$id",
												params: { id: item.id },
												search: { tab: "deployments" },
											});
										return (
											<tr
												role="row"
												tabIndex={0}
												class="table-row cursor-pointer focus-visible:outline-primary"
												onClick={goToDetail}
												onKeyDown={(e) => {
													if (e.key === "Enter" || e.key === " ") {
														e.preventDefault();
														goToDetail();
													}
												}}
											>
												<td role="cell" class="flex-4 flex items-center justify-start min-w-0">
													<span class="truncate font-medium text-white">{item.name}</span>
												</td>
												<td role="cell" class="flex-2 flex items-center justify-center min-w-0">
													<StatusChip status={item.connected ? "connected" : "unreachable"} />
												</td>
												<td role="cell" class="flex-3 flex items-center justify-start min-w-0">
													<span class="text-grey">
														{item.connected
															? "-"
															: item.lastSeen
																? formatRelativeTime(item.lastSeen)
																: "Never"}
													</span>
												</td>
												<td role="cell" class="flex-3 flex items-center justify-start min-w-0">
													<CopyableField
														variant={CopyableFieldVariant.Text}
														value={item.id}
														class="truncate"
														innerClass="text-grey font-log text-xs"
													/>
												</td>
											</tr>
										);
									}}
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
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/runners/")({
	validateSearch: (search: Record<string, unknown>): { page?: string; count?: string } => ({
		page: (search.page as string) || undefined,
		count: (search.count as string) || undefined,
	}),
	component: ListRunnersPage,
});
