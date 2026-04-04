import { createFileRoute } from "@tanstack/solid-router";
import { useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createMemo, createResource, ErrorBoundary, Show, Suspense } from "solid-js";
import { ListApiTokensResponse } from "~/bindings";
import {
	ButtonVariant,
	EmptyState,
	Link,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	Pagination,
	Table,
	useToast,
} from "~/components";
import Button from "~/components/button";
import { LoadingSpinner } from "~/components/loading-spinner";
import { useAuthState, createPaginationState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { formatRelativeTime } from "~/utils/func";
import { httpRequest } from "~/utils/http-request";

const ListApiTokens = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const navigate = useNavigate();
	const search = Route.useSearch();
	const pagination = createPaginationState({
		search: () => search(),
		navigate,
	});

	const fetchParams = createMemo(() => {
		return [authState(), workspaceId(), pagination.page(), pagination.count()] as const;
	});

	const [apiTokens] = createResource(fetchParams, async ([auth, wsId, page, count]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn") {
			return { tokens: [] };
		}

		const response = await httpRequest<ListApiTokensResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/user/api-token?page=${page}&count=${count}`,
			{
				method: "GET",
			}
		);

		if (!response.ok) {
			toast("Failed to fetch API Tokens", "error");
			return { tokens: [] };
		}

		pagination.setTotalCount(Number(response.headers.get("x-total-count") ?? 0));

		return { tokens: response.data.tokens };
	});

	return (
		<>
			<Title>API Tokens | Patr</Title>
			<PageContainer>
				<PageContainerHead
					breadcrumbs={[
						{
							label: "Profile",
							url: "/profile",
						},
						{
							label: "API Tokens",
						},
					]}
					subText="Programmatic access tokens for the Patr API"
					actions={() => (
						<Show when={(apiTokens()?.tokens?.length ?? 0) > 0}>
							<Link
								href="/profile/api-tokens/new"
								buttonVariant={ButtonVariant.Outlined}
								external={false}
							>
								Create API Token
							</Link>
						</Show>
					)}
				/>
				<PageContainerBody class="flex flex-col justify-between">
					<ErrorBoundary
						fallback={(err, reset) => (
							<div class="flex flex-col items-center justify-center gap-4 py-16">
								<p class="text-error text-sm">Error loading API tokens: {err.message}</p>
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
									<span class="text-sm">Loading API tokens...</span>
								</div>
							}
						>
							<Show
								when={(apiTokens()?.tokens?.length ?? 0) > 0}
								fallback={
									<EmptyState
										title="No API Tokens Created"
										description="Create a token to access the Patr API programmatically."
										action={
											<Link
												href="/profile/api-tokens/new"
												buttonVariant={ButtonVariant.Outlined}
												external={false}
											>
												Create API Token
											</Link>
										}
									/>
								}
							>
								<Table
									column_grids={["flex-4", "flex-4", "flex-4"]}
									headings={["Token Name", "Created", "Expiry"]}
									rows={apiTokens()?.tokens || []}
									renderRow={(token) => {
										const goToDetail = () => navigate({ to: `/profile/api-tokens/${token.id}` });
										return (
											<tr
												role="row"
												tabIndex={0}
												onClick={goToDetail}
												onKeyDown={(e) => {
													if (e.key === "Enter" || e.key === " ") {
														e.preventDefault();
														goToDetail();
													}
												}}
												class="table-row cursor-pointer focus-visible:outline-primary"
											>
												<td role="cell" class="flex-4 flex items-center justify-start min-w-0">
													<span class="truncate font-medium text-white">{token.name}</span>
												</td>
												<td role="cell" class="flex-4 flex items-center justify-start min-w-0">
													<span class="text-grey">
														{formatRelativeTime(token.created) || "Unknown"}
													</span>
												</td>
												<td role="cell" class="flex-4 flex items-center justify-start min-w-0">
													<span class="text-grey">
														{token.tokenExp ? formatRelativeTime(token.tokenExp) : "Never"}
													</span>
												</td>
											</tr>
										);
									}}
								/>
								<Pagination
									state={pagination}
									loading={apiTokens.loading}
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

export const Route = createFileRoute("/_logged-in/_workspaced/profile/api-tokens/")({
	validateSearch: (search: Record<string, unknown>): { page?: string; count?: string } => ({
		page: (search.page as string) || undefined,
		count: (search.count as string) || undefined,
	}),
	component: ListApiTokens,
});
