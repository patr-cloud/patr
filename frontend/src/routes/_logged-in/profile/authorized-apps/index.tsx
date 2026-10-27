import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { useQueryClient } from "@tanstack/solid-query";
import { createEffect, createSignal, ErrorBoundary, Show, Suspense } from "solid-js";
import {
	Button,
	ButtonVariant,
	DeleteModal,
	EmptyState,
	LoadingSpinner,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	Pagination,
	Table,
	useToast,
} from "~/components";
import { createPaginationState, recoverFromOutOfBounds, paginationSearch } from "~/hooks";
import { useOAuthGrantsQuery } from "~/hooks/fetch";
import { UserOAuthGrant, WithId } from "~/bindings";
import { oauthGrantKeys } from "~/hooks/query-keys";
import { formatRelativeTime } from "~/utils/func";
import { httpRequest } from "~/utils/http-request";
import { SCOPE_COPY } from "~/utils/oauth";

const GrantRow = (props: { grant: WithId<UserOAuthGrant>; onRevoked: () => void }) => {
	const toast = useToast();
	const [revokeOpen, setRevokeOpen] = createSignal(false);

	const onClickRevoke = async () => {
		const response = await httpRequest<void>(
			`${import.meta.env.VITE_BASE_URL}/api/user/oauth-grant/${props.grant.id}`,
			{ method: "DELETE" }
		);

		if (!response.ok) {
			toast(`Failed to revoke ${props.grant.clientName}'s access`, "error");
			return;
		}

		toast(`${props.grant.clientName} can no longer access your account`, "success");
		props.onRevoked();
	};

	return (
		<tr role="row" class="table-row">
			<td role="cell" class="flex-4 flex items-center justify-start min-w-0 gap-3">
				<img src={props.grant.clientLogoUrl} alt="" class="size-6 rounded-xs object-contain" />
				<span class="truncate font-medium text-white">{props.grant.clientName}</span>
			</td>
			<td role="cell" class="flex-4 flex items-center justify-start min-w-0">
				<span class="truncate text-grey">
					{props.grant.scope
						.split(" ")
						.map((scope) => SCOPE_COPY[scope]?.title ?? scope)
						.join(", ")}
				</span>
			</td>
			<td role="cell" class="flex-3 flex items-center justify-start min-w-0">
				<span class="text-grey">{formatRelativeTime(props.grant.lastUsed) || "Unknown"}</span>
			</td>
			<td role="cell" class="flex-2 flex items-center justify-end min-w-0">
				<DeleteModal
					title={`Revoke ${props.grant.clientName}'s Access`}
					message="The app will be signed out of your account."
					resourceName={props.grant.clientName}
					onClickDelete={onClickRevoke}
					isOpen={revokeOpen}
					setIsOpen={setRevokeOpen}
					renderTrigger={(open) => (
						<Button variant={ButtonVariant.Outlined} onClick={() => open?.(true)}>
							Revoke
						</Button>
					)}
				/>
			</td>
		</tr>
	);
};

const AuthorizedApps = () => {
	const navigate = useNavigate();
	const search = Route.useSearch();
	const queryClient = useQueryClient();
	const pagination = createPaginationState({
		search: () => search(),
		navigate,
	});

	const grantsQuery = useOAuthGrantsQuery(
		() => search().page?.toString(),
		() => search().count?.toString()
	);

	createEffect(() => {
		const totalCount = grantsQuery.data?.totalCount;
		if (totalCount !== undefined) {
			pagination.setTotalCount(totalCount);
		}
	});

	recoverFromOutOfBounds(
		() => grantsQuery.isError,
		() => grantsQuery.error?.message,
		pagination
	);

	return (
		<>
			<Title>Authorized Apps | Patr</Title>
			<PageContainer>
				<PageContainerHead
					breadcrumbs={[
						{
							label: "Profile",
							url: "/profile",
						},
						{
							label: "Authorized Apps",
						},
					]}
					subText="Apps you've allowed to act on your behalf"
				/>
				<PageContainerBody class="flex flex-col justify-between">
					<ErrorBoundary
						fallback={(err, reset) => (
							<div class="flex flex-col items-center justify-center gap-4 py-16">
								<p class="text-error text-sm">Error loading authorized apps: {err.message}</p>
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
									<span class="text-sm">Loading authorized apps...</span>
								</div>
							}
						>
							<Show
								when={(grantsQuery.data?.grants?.length ?? 0) > 0}
								fallback={
									<EmptyState
										title="No Authorized Apps"
										description="Apps you sign into with your Patr account will show up here."
									/>
								}
							>
								<Table
									column_grids={["flex-4", "flex-4", "flex-3", "flex-2"]}
									headings={["App", "Approved", "Last Used", ""]}
									rows={grantsQuery.data?.grants || []}
									renderRow={(grant) => (
										<GrantRow
											grant={grant}
											onRevoked={() =>
												queryClient.invalidateQueries({ queryKey: oauthGrantKeys.all() })
											}
										/>
									)}
								/>
								<Pagination
									state={pagination}
									loading={grantsQuery.isFetching}
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

export const Route = createFileRoute("/_logged-in/profile/authorized-apps/")({
	validateSearch: paginationSearch,
	component: AuthorizedApps,
});
