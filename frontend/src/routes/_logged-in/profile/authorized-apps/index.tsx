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
import { createPaginationState } from "~/hooks";
import { useOAuthGrantsQuery } from "~/hooks/fetch";
import { oauthGrantKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";
import { formatRelativeTime } from "~/utils/func";

/// Turns `openid profile email` into something a person can read.
const describeScope = (scope: string) => {
	const labels: Record<string, string> = {
		openid: "who you are",
		profile: "your name",
		email: "your email address",
		offline_access: "ongoing access",
	};

	const described = scope
		.split(" ")
		.filter((entry) => entry.length > 0)
		.map((entry) => labels[entry] ?? entry);

	return described.length > 0 ? described.join(", ") : "nothing";
};

const AuthorizedApps = () => {
	const navigate = useNavigate();
	const search = Route.useSearch();
	const queryClient = useQueryClient();
	const toast = useToast();
	const pagination = createPaginationState({
		search: () => search(),
		navigate,
	});

	const grantsQuery = useOAuthGrantsQuery(
		() => search().page,
		() => search().count
	);

	const [revoking, setRevoking] = createSignal<{ id: string; name: string } | null>(null);
	const [isRevokeModalOpen, setIsRevokeModalOpen] = createSignal(false);

	createEffect(() => {
		const totalCount = grantsQuery.data?.totalCount;
		if (totalCount !== undefined) {
			pagination.setTotalCount(totalCount);
		}
	});

	const onClickRevoke = async () => {
		const target = revoking();
		if (!target) {
			return;
		}

		const response = await httpRequest<void>(`${import.meta.env.VITE_BASE_URL}/api/user/oauth-grant/${target.id}`, {
			method: "DELETE",
		});

		if (!response.ok) {
			console.error("Failed to revoke access:", response.data.error);
			toast(`Failed to revoke ${target.name}'s access`, "error");
			return;
		}

		toast(`${target.name} can no longer access your account`, "success");
		setRevoking(null);
		await queryClient.invalidateQueries({ queryKey: oauthGrantKeys.all() });
	};

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
									headings={["App", "Can See", "Last Used", ""]}
									rows={grantsQuery.data?.grants || []}
									renderRow={(grant) => (
										<tr role="row" class="table-row">
											<td
												role="cell"
												class="flex-4 flex items-center justify-start min-w-0 gap-3"
											>
												<img
													src={grant.clientLogoUrl}
													alt=""
													class="size-6 rounded-xs object-contain"
												/>
												<span class="truncate font-medium text-white">{grant.clientName}</span>
											</td>
											<td role="cell" class="flex-4 flex items-center justify-start min-w-0">
												<span class="truncate text-grey">{describeScope(grant.scope)}</span>
											</td>
											<td role="cell" class="flex-3 flex items-center justify-start min-w-0">
												<span class="text-grey">
													{formatRelativeTime(grant.lastUsed) || "Never"}
												</span>
											</td>
											<td role="cell" class="flex-2 flex items-center justify-end min-w-0">
												<Button
													variant={ButtonVariant.Outlined}
													onClick={() => {
														setRevoking({ id: grant.id, name: grant.clientName });
														setIsRevokeModalOpen(true);
													}}
												>
													Revoke
												</Button>
											</td>
										</tr>
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
					<DeleteModal
						title="Revoke Access"
						onClickDelete={onClickRevoke}
						resourceName={revoking()?.name || ""}
						isOpen={isRevokeModalOpen}
						setIsOpen={setIsRevokeModalOpen}
					/>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/profile/authorized-apps/")({
	validateSearch: (search: Record<string, unknown>): { page?: string; count?: string } => ({
		page: (search.page as string) || undefined,
		count: (search.count as string) || undefined,
	}),
	component: AuthorizedApps,
});
