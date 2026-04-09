import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createMemo, createResource, createSignal, ErrorBoundary, Suspense, Show } from "solid-js";
import { FiAlertCircle } from "solid-icons/fi";
import {
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	Table,
	Button,
	ButtonVariant,
	useToast,
	Modal,
	ModalContainer,
	CopyableField,
	CopyableFieldVariant,
	EmptyState,
	Link,
	Pagination,
	StatusChip,
	LoadingSpinner,
} from "~/components";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import { GetDomainInfoInWorkspaceResponse } from "~/bindings";
import { EventT } from "~/utils/types";
import { useIsAllowed, createPaginationState } from "~/hooks";

// Type definitions based on API bindings
type WorkspaceDomain = {
	id: string;
	name: string;
	nameserverType: string;
	isVerified: boolean;
};

type GetDomainsForWorkspaceResponse = {
	domains: WorkspaceDomain[];
};

const DNSRecords = (props: { domainId: string; domainName: string; closeFn: (prev: boolean) => void }) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const [loading, setLoading] = createSignal(false);

	const verificationRecord = {
		type: "TXT",
		name: `_patr-verify.${props.domainName}`,
		target: props.domainId,
	};

	const onVerifyClick = async (_: EventT<MouseEvent, HTMLButtonElement>) => {
		setLoading(true);

		const auth = authState();
		const wsId = workspaceId();
		const domainId = props.domainId;

		if (!wsId || !auth || auth.type !== "LoggedIn" || !domainId) {
			toast("Unable to verify domain", "error");
			return;
		}

		const verifyResp = await httpRequest<GetDomainInfoInWorkspaceResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/domain/${domainId}/verify`,
			{
				method: "POST",
			}
		);

		if (!verifyResp.ok) {
			console.error("Failed to verify domain:", verifyResp.data.error);
			toast("Failed to verify domain", "error");
			setLoading(false);
			return;
		}

		setLoading(false);
		toast("Domain verification initiated", "success");
		props.closeFn(false);
	};

	return (
		<ModalContainer
			style={{
				width: "60rem",
			}}
			onClick={(e) => {
				e.stopPropagation();
			}}
			closeFn={props.closeFn}
		>
			<p class="text-primary text-md mb-2">To verify domain, add the following DNS record:</p>
			<div class="mb-4">
				<Table
					column_grids={["flex-2", "flex-5", "flex-5"]}
					headings={["Type", "Name", "Value"]}
					rows={[verificationRecord]}
					renderRow={(record) => (
						<tr class="table-row text-sm">
							<td class="flex-2 flex items-center justify-center">
								<CopyableField variant={CopyableFieldVariant.Text} value={record.type} />
							</td>
							<td class="flex-5 flex items-center justify-center min-w-0">
								<CopyableField
									variant={CopyableFieldVariant.Text}
									value={record.name}
									innerClass="truncate max-w-full"
								/>
							</td>
							<td class="flex-5 flex items-center justify-center min-w-0">
								<CopyableField
									variant={CopyableFieldVariant.Text}
									value={record.target}
									innerClass="truncate max-w-full"
								/>
							</td>
						</tr>
					)}
				/>
				<p class="text-gray-400 text-xs mt-2">
					After adding the DNS record, it may take up to 24 hours to propagate (but is usually done within a
					few minutes).
				</p>

				<div class="w-full flex items-center justify-end mt-4">
					<Button variant={ButtonVariant.Contained} onClick={onVerifyClick} disabled={loading()}>
						{loading() ? "Verifying..." : "Verify"}
					</Button>
				</div>
			</div>
		</ModalContainer>
	);
};

const VerificationIcon = (props: { domain: WorkspaceDomain }) => {
	return (
		<Show when={!props.domain.isVerified} fallback={null}>
			<Modal
				renderTrigger={(setOpen) => (
					<Button
						variant={ButtonVariant.Plain}
						aria-label="Verify domain"
						onClick={(e: EventT<MouseEvent, HTMLButtonElement>) => {
							e.stopPropagation();
							setOpen(true);
						}}
					>
						<FiAlertCircle
							size={22}
							class="text-yellow-500 cursor-pointer hover:bg-white/10 transition-colors rounded p-1"
						/>
					</Button>
				)}
				renderModalContent={(close) => (
					<DNSRecords domainId={props.domain.id} domainName={props.domain.name} closeFn={close} />
				)}
			/>
		</Show>
	);
};

const ListDomainsPage = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const navigate = useNavigate();
	const toast = useToast();

	const isCreateAllowed = useIsAllowed("domain", "add");
	const search = Route.useSearch();
	const pagination = createPaginationState({
		search: () => search(),
		navigate,
	});

	const fetchParams = createMemo(() => {
		return [authState(), workspaceId(), pagination.page(), pagination.count()] as const;
	});

	const [domains] = createResource(fetchParams, async ([auth, wsId, page, count]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn") {
			return { domains: [] };
		}

		const response = await httpRequest<GetDomainsForWorkspaceResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/domain?page=${page}&count=${count}`,
			{
				method: "GET",
			}
		);

		if (!response.ok) {
			console.error("Failed to fetch domains:", response.data.error);
			toast("Failed to fetch domains", "error");
			return { domains: [] };
		}

		pagination.setTotalCount(Number(response.headers.get("x-total-count") ?? 0));

		console.log("Fetched domains:", response.data);
		return { domains: response.data.domains || [] };
	});

	return (
		<>
			<Title>Domains | Patr</Title>
			<PageContainer>
				<PageContainerHead
					breadcrumbs={[
						{
							label: "Domains",
						},
					]}
					subText="Configure custom domains to route traffic to your deployments."
					actions={() => (
						<Show when={isCreateAllowed() && (domains()?.domains?.length ?? 0) > 0}>
							<Link href="/domains/new" buttonVariant={ButtonVariant.Outlined} external={false}>
								Add Domain
							</Link>
						</Show>
					)}
				/>
				<PageContainerBody class="flex flex-col justify-between">
					<ErrorBoundary
						fallback={(err, reset) => (
							<div class="flex flex-col items-center justify-center gap-4 py-16">
								<p class="text-error text-sm">Error loading domains: {err.message}</p>
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
									<span class="text-sm">Loading domains...</span>
								</div>
							}
						>
							<Show
								when={(domains()?.domains?.length ?? 0) > 0}
								fallback={
									<EmptyState
										title="No Domains Added"
										description={
											isCreateAllowed()
												? "Register a custom domain to route traffic to your deployments."
												: undefined
										}
										action={
											isCreateAllowed() ? (
												<Link
													href="/domains/new"
													buttonVariant={ButtonVariant.Outlined}
													external={false}
												>
													Add Domain
												</Link>
											) : undefined
										}
									/>
								}
							>
								<Table
									column_grids={["flex-5", "flex-3", "flex-4"]}
									rows={domains()?.domains || []}
									headings={["Domain", "Type", "Status"]}
									renderRow={(item) => {
										const goToDetail = () => navigate({ to: `/domains/${item.id}` });
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
												<td role="cell" class="flex-5 flex items-center justify-start min-w-0">
													<span class="truncate font-medium text-white">{item.name}</span>
												</td>
												<td role="cell" class="flex-3 flex items-center justify-start min-w-0">
													<span class="text-grey">
														{item.nameserverType === "patr" ? "Patr Managed" : "External"}
													</span>
												</td>
												<td role="cell" class="flex-4 flex items-center justify-start min-w-0">
													<div class="flex items-center gap-2">
														<StatusChip status={item.isVerified ? "running" : "stopped"} />
														<span class="text-sm">
															{item.isVerified ? "Verified" : "Not Verified"}
														</span>
														<Show when={!item.isVerified}>
															<VerificationIcon domain={item} />
														</Show>
													</div>
												</td>
											</tr>
										);
									}}
								/>
								<Pagination
									state={pagination}
									loading={domains.loading}
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

export const Route = createFileRoute("/_logged-in/_workspaced/domains/")({
	validateSearch: (search: Record<string, unknown>): { page?: string; count?: string } => ({
		page: (search.page as string) || undefined,
		count: (search.count as string) || undefined,
	}),
	component: ListDomainsPage,
});
