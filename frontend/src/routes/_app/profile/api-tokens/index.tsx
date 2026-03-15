import { createFileRoute } from "@tanstack/solid-router";
import { useNavigate } from "@tanstack/solid-router";
import { createMemo, createResource, Show, Suspense } from "solid-js";
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
			console.error("Failed to fetch API Tokens:", response.data.error);
			toast("Failed to fetch API Tokens", "error");
			return { tokens: [] };
		}

		pagination.setTotalCount(Number(response.headers.get("x-total-count") ?? 0));

		return { tokens: response.data.tokens };
	});

	return (
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
				subText="API Tokens"
				actions={() => (
					<Link href="/profile/api-tokens/new" buttonVariant={ButtonVariant.Contained} external={false}>
						Create API Token
					</Link>
				)}
			/>
			<PageContainerBody class="flex flex-col justify-between gap-8">
				<Suspense fallback={<div>Loading API Tokens...</div>}>
					<Show
						when={(apiTokens()?.tokens?.length ?? 0) > 0}
						fallback={<EmptyState title="No API Tokens Created" />}
					>
						<Table
							column_grids={["flex-4", "flex-4", "flex-4"]}
							headings={["Token Name", "Created", "Expiry"]}
							rows={apiTokens()?.tokens || []}
							renderRow={(token) => (
								<tr
									onClick={() => {
										navigate({ to: `/profile/api-tokens/${token.id}` });
									}}
									class="table-row cursor-pointer"
								>
									<td class="flex-4 flex items-center justify-center">{token.name}</td>
									<td class="flex-4 flex items-center justify-center">
										{formatRelativeTime(token.created) || "Unknown"}
									</td>
									<td class="flex-4 flex items-center justify-center">
										{token.tokenExp ? formatRelativeTime(token.tokenExp) : "Never"}
									</td>
								</tr>
							)}
						/>
						<Pagination
							state={pagination}
							loading={apiTokens.loading}
							showPageSizeSelector={false}
							showGoToPage={false}
						/>
					</Show>
				</Suspense>
			</PageContainerBody>
		</PageContainer>
	);
};

export const Route = createFileRoute("/_app/profile/api-tokens/")({
	validateSearch: (search: Record<string, unknown>): { page?: string; count?: string } => ({
		page: (search.page as string) || undefined,
		count: (search.count as string) || undefined,
	}),
	component: ListApiTokens,
});
