import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { createMemo, createResource, createSignal, ErrorBoundary, Suspense, For, Show } from "solid-js";
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
	CopyableField,
	CopyableFieldVariant,
	EmptyState,
	Pagination,
} from "~/components";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import { ModalContainer } from "~/components/modal";
import { GetDomainInfoInWorkspaceResponse, GetVerificationRecordsForDomainResponse } from "~/bindings";
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

const DNSRecords = (props: { domainId: string; closeFn: (prev: boolean) => void }) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const [loading, setLoading] = createSignal(false);

	const fetchParams = createMemo(() => {
		return [authState(), workspaceId(), props.domainId] as const;
	})

	const [dnsRecord] = createResource(fetchParams, async ([auth, wsId, domainId]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn") {
			throw new Error("Not authenticated or workspace ID missing");
		}

		const response = await httpRequest<GetVerificationRecordsForDomainResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/domain/${domainId}/verification-records`,
			{
				method: "GET",
			}
		)

		if (!response.ok) {
			console.error("Failed to fetch DNS records:", response.data.error);
			throw new Error("Failed to fetch DNS records");
		}

		return { records: response.data.verificationRecords || [] };
	})

	const onVerifyClick = async (e: EventT<MouseEvent, HTMLButtonElement>) => {
		setLoading(true);

		const auth = authState();
		const wsId = workspaceId();
		const domainId = props.domainId;

		if (!wsId || !auth || auth.type !== "LoggedIn" || !domainId) {
			toast("Unable to verify domain", "error");
			return
		}

		const verifyResp = await httpRequest<GetDomainInfoInWorkspaceResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/domain/${domainId}/verify`,
			{
				method: "POST",
			}
		)

		if (!verifyResp.ok) {
			console.error("Failed to verify domain:", verifyResp.data.error);
			toast("Failed to verify domain", "error");
			setLoading(false);
			return
		}

		setLoading(false);
		toast("Domain verification initiated", "success");
		props.closeFn(false);
	}

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
			<ErrorBoundary
				fallback={(err, reset) => (
					<div class="text-white">
						<p>Error loading DNS records: {err.message}</p>
						<Button variant={ButtonVariant.Contained} onClick={reset}>
							Retry
						</Button>
					</div>
				)}
			>
				<Suspense
					fallback={
						<div class="flex items-center justify-center py-8">
							<div class="text-gray-400">Loading DNS records...</div>
						</div>
					}
				>
					<Show when={dnsRecord()?.records.length}>
						<p class="text-primary text-md mb-2">To verify domain, add the following DNS records:</p>
						<div class="mb-4">
							<Table
								column_grids={["flex-2", "flex-4", "flex-4"]}
								headings={["Type", "Name", "Value"]}
								rows={dnsRecord()!.records}
								renderRow={(record) => (
									<tr class="table-row text-sm">
										<td class="flex-2 flex items-center justify-center">
											<CopyableField variant={CopyableFieldVariant.Text} value={record.type} />
										</td>
										<td class="flex-4 flex items-center justify-center min-w-0">
											<CopyableField
												variant={CopyableFieldVariant.Text}
												value={record.name}
												innerClass="truncate max-w-full"
											/>
										</td>
										<td class="flex-4 flex items-center justify-center min-w-0">
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
								After adding the DNS record, it may take up to 48 hours to propagate.
							</p>

							<div class="w-full flex items-center justify-end mt-4">
								<Button variant={ButtonVariant.Contained} onClick={onVerifyClick} disabled={loading()}>
									{loading() ? "VERIFYING..." : "VERIFY"}
								</Button>
							</div>
						</div>
					</Show>
				</Suspense>
			</ErrorBoundary>
		</ModalContainer>
	)
};

const VerificationIcon = (props: { domain: WorkspaceDomain }) => {
	if (props.domain.isVerified) {
		return null;
	}

	return (
		<Modal
			renderTrigger={(setOpen) => (
				<Button
					variant={ButtonVariant.Plain}
					onClick={(e: EventT<MouseEvent, HTMLButtonElement>) => {
						e.stopPropagation();
						setOpen(true)
					}}
				>
					<FiAlertCircle
						size={22}
						class="text-yellow-500 cursor-pointer hover:bg-white/10 transition-colors rounded p-1"
					/>
				</Button>
			)}
			renderModalContent={(close) => <DNSRecords domainId={props.domain.id} closeFn={close} />}
		/>
	)
};

const ListDomainsPage = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const navigate = useNavigate();
	const toast = useToast();

	const isCreateAllowed = useIsAllowed("domain", "create");
	const search = Route.useSearch();
	const pagination = createPaginationState({
		search: () => search(),
		navigate,
	})

	const fetchParams = createMemo(() => {
		return [authState(), workspaceId(), pagination.page(), pagination.count()] as const;
	})

	const [domains] = createResource(fetchParams, async ([auth, wsId, page, count]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn") {
			return { domains: [] };
		}

		const response = await httpRequest<GetDomainsForWorkspaceResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/domain?page=${page}&count=${count}`,
			{
				method: "GET",
			}
		)

		if (!response.ok) {
			console.error("Failed to fetch domains:", response.data.error);
			toast("Failed to fetch domains", "error");
			return { domains: [] };
		}

		pagination.setTotalCount(Number(response.headers.get("x-total-count") ?? 0));

		console.log("Fetched domains:", response.data);
		return { domains: response.data.domains || [] };
	})

	return (
		<PageContainer>
			<PageContainerHead
				breadcrumbs={[
					{
						label: "Domains",
					},
				]}
				subText="Configure custom domains to route traffic to your deployments."
				actions={() => {
					if (!isCreateAllowed()) return null;
					return (
						<div class="ml-auto">
							<Button
								class="cursor-pointer"
								variant={ButtonVariant.Plain}
								onClick={() => navigate({ to: "/domains/new" })}
							>
								Add Domain
							</Button>
						</div>
					)
				}}
			/>
			<PageContainerBody class="flex flex-col justify-between">
				<ErrorBoundary
					fallback={(err, reset) => (
						<div>
							<p>Error loading domains: {err.message}</p>
							<button onClick={reset}>Retry</button>
						</div>
					)}
				>
					<Suspense fallback={<div>Loading domains...</div>}>
						<Show
							when={(domains()?.domains?.length ?? 0) > 0}
							fallback={<EmptyState title="No Domain Added" />}
						>
							<Table
								column_grids={["flex-3", "flex-3", "flex-3"]}
								rows={domains()?.domains || []}
								headings={["Domain Name", "Type", "Verified"]}
								renderRow={(item) => (
									<tr
										onClick={() => navigate({ to: `/domains/${item.id}` })}
										class="table-row cursor-pointer"
									>
										<td class="flex-3 flex items-center justify-center">{item.name}</td>
										<td class="flex-3 flex items-center justify-center">{item.nameserverType}</td>
										<td class="flex-3 flex items-center justify-center">
											<div class="flex items-center gap-2">
												{item.isVerified ? (
													<span class="text-green-500">✓ Verified</span>
												) : (
													<>
														<span class="text-yellow-500">Not Verified</span>
														<VerificationIcon domain={item} />
													</>
												)}
											</div>
										</td>
									</tr>
								)}
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
	)
};

export const Route = createFileRoute("/_logged-in/_workspaced/domains/")({
	validateSearch: (search: Record<string, unknown>): { page?: string; count?: string } => ({
		page: (search.page as string) || undefined,
		count: (search.count as string) || undefined,
	}),
	component: ListDomainsPage,
});
