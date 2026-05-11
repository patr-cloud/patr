import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createSignal, ErrorBoundary, Show } from "solid-js";
import {
	CreateManagedURLRequest,
	CreateManagedURLResponse,
	DeleteDomainInWorkspaceResponse,
	GetDomainInfoInWorkspaceResponse,
} from "~/bindings";
import {
	Button,
	ButtonVariant,
	CopyableField,
	CopyableFieldVariant,
	DeleteModal,
	Input,
	InputDropdown,
	LoadingSpinner,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	Table,
	useToast,
} from "~/components";
import { createAuthenticatedAction, createFormAction } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { useDomainInfoQuery, useManagedUrlsQuery } from "~/hooks/fetch";
import { domainKeys, managedUrlKeys } from "~/hooks/query-keys";
import { useQueryClient } from "@tanstack/solid-query";
import { httpRequest } from "~/utils/http-request";
import DeploymentOption from "./-components/deployment-option";
import ManageUrlRow from "./-components/managed-url-component";

type urlTypeT = "redirect" | "proxyDeployment";

const DomainInfo = () => {
	const params = Route.useParams();

	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const navigate = useNavigate();
	const queryClient = useQueryClient();

	const [subDomain, setSubDomain] = createSignal("");
	const [path, setPath] = createSignal("");
	const [urlType, setUrlType] = createSignal<urlTypeT | null>(null);
	const [target, setTarget] = createSignal<string | null>(null);
	const [deploymentPort, setDeploymentPort] = createSignal<number | null>(null);

	const domainInfoQuery = useDomainInfoQuery(() => params().id);
	const managedUrlsQuery = useManagedUrlsQuery(() => params().id);

	const refetchDomainInfo = () => {
		const wsId = workspaceId();
		if (wsId) {
			queryClient.invalidateQueries({ queryKey: domainKeys.detail(wsId, params().id) });
		}
	};

	const refetchManagedUrls = () => {
		const wsId = workspaceId();
		if (wsId) {
			queryClient.invalidateQueries({ queryKey: managedUrlKeys.list(wsId, params().id) });
		}
	};

	const { onSubmit: onSubmitCreateManagedUrl, isLoading: isCreatingManagedUrl } = createFormAction(
		async ({ workspaceId }) => {
			const domainId = params().id;

			if (!domainId) {
				toast("Domain ID is missing", "error");
				return;
			}

			const urlTypeVal = urlType();
			const targetVal = target();
			if (!urlTypeVal || !subDomain() || !targetVal) {
				toast("Please fill in all required fields", "error");
				return;
			}

			const base = {
				domainId,
				subDomain: subDomain(),
				path: path(),
			};

			let requestBody: CreateManagedURLRequest;
			switch (urlTypeVal) {
				case "proxyDeployment":
					requestBody = {
						...base,
						type: "proxyDeployment",
						deploymentId: targetVal,
						port: deploymentPort() || 80,
					};
					break;
				case "redirect":
					requestBody = {
						...base,
						type: "redirect",
						url: targetVal,
						permanentRedirect: false,
						httpOnly: false,
					};
					break;
			}

			const response = await httpRequest<CreateManagedURLResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/infrastructure/managed-url`,
				{
					method: "POST",
					body: JSON.stringify(requestBody),
				}
			);

			if (!response.ok) {
				console.error("Failed to create managed URL:", response.data.error);
				toast("Failed to create managed URL", "error");
				return;
			}

			toast("Managed URL created successfully", "success");
			setSubDomain("");
			setPath("");
			setUrlType(null);
			setTarget(null);
			setDeploymentPort(null);
			refetchManagedUrls();
		}
	);

	const { execute: onVerifyClick, isLoading: verifyLoading } = createAuthenticatedAction(async ({ workspaceId }) => {
		const domainId = params().id;

		if (!domainId) {
			toast("Unable to verify domain", "error");
			return;
		}

		const verifyResp = await httpRequest<GetDomainInfoInWorkspaceResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/domain/${domainId}/verify`,
			{ method: "POST" }
		);

		if (!verifyResp.ok) {
			console.error("Failed to verify domain:", verifyResp.data.error);
			toast("Failed to verify domain", "error");
			return;
		}

		toast("Domain verification initiated", "success");
		refetchDomainInfo();
	});

	const { execute: onClickDelete, isLoading: deleteLoading } = createAuthenticatedAction(
		async ({ accessToken, workspaceId }) => {
			const domainId = params().id;

			if (!workspaceId || !accessToken || !domainId) {
				toast("Unable to delete domain", "error");
				return;
			}

			const deleteDomainResp = await httpRequest<DeleteDomainInWorkspaceResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/domain/${domainId}`,
				{ method: "DELETE" }
			);

			if (!deleteDomainResp.ok) {
				console.error("Failed to delete domain:", deleteDomainResp.data.error);
				if (deleteDomainResp.data.error === "resourceInUse") {
					toast("Cannot delete domain: Domain is in use by managed URL(s)", "error");
					return;
				}
				toast("Failed to delete domain", "error");
				return;
			}

			toast("Domain deleted successfully", "success");
			navigate({ to: "/domains" });
		}
	);

	const urlInput = () => {
		const urlTypeVal = urlType();
		switch (urlTypeVal) {
			case "proxyDeployment":
				return (
					<DeploymentOption
						deployment={target()}
						onSelectDeployment={(value) => setTarget(value)}
						port={deploymentPort() || 80}
						onPortChange={(port) => setDeploymentPort(port)}
					/>
				);
			default:
				return <Input disabled={true} placeholder="Select URL Type" class="flex-4" />;
		}
	};

	return (
		<>
			<Title>Domain Details | Patr</Title>
			<PageContainer>
				<ErrorBoundary
					fallback={(err, reset) => (
						<div class="flex flex-col items-center justify-center gap-4 py-16">
							<p class="text-error text-sm">Error loading domain details: {err.message}</p>
							<button class="text-primary hover:underline text-sm" onClick={reset}>
								Retry
							</button>
						</div>
					)}
				>
					<Show
						when={!domainInfoQuery.isPending}
						fallback={
							<div class="flex items-center justify-center gap-2 py-16 text-grey">
								<LoadingSpinner size={20} />
								<span class="text-sm">Loading domain...</span>
							</div>
						}
					>
						<PageContainerHead
							breadcrumbs={[
								{
									label: "Domains",
									url: "/domains",
								},
								{
									label: domainInfoQuery.data?.name || "Loading...",
								},
							]}
							subText="Configure custom domains to route traffic to your deployments."
							actions={() => (
								<div class="flex items-center justify-center gap-2">
									<DeleteModal
										isLoading={deleteLoading()}
										title="Delete Domain"
										onClickDelete={(e) => {
											e.preventDefault();
											onClickDelete();
										}}
										resourceName={domainInfoQuery.data?.name || ""}
									/>
									{!domainInfoQuery.data?.isVerified ? (
										<Button
											type="button"
											onClick={(e) => {
												e.preventDefault();
												onVerifyClick();
											}}
											variant={ButtonVariant.Contained}
											loading={verifyLoading()}
											loadingContent={() => <>Verifying...</>}
										>
											Verify
										</Button>
									) : undefined}
								</div>
							)}
						/>
						<PageContainerBody>
							<form class="mb-2 p-lg bg-secondary-light rounded-xs" onSubmit={onSubmitCreateManagedUrl}>
								<h1 class="text-lg mb-3">Create New Managed URL</h1>
								<div class="flex flex-col items-start justify-center gap-2 w-full">
									<div class="flex items-center justify-center gap-3 w-full">
										<Input
											onInput={(e) => setSubDomain(e.currentTarget.value)}
											value={subDomain()}
											styleVariant="medium"
											class="flex-2"
											placeholder="Sub-domain"
										/>
										<span class="h-full">.</span>
										<Input
											disabled={true}
											value={domainInfoQuery.data?.name}
											class="flex-2"
											placeholder="Domain"
										/>
										<span>/</span>
										<Input
											styleVariant="medium"
											onInput={(e) => setPath(e.currentTarget.value)}
											value={path()}
											class="flex-2"
											placeholder="Path"
										/>
									</div>
									<p class="mx-2">Will point to</p>
									<div class="flex items-center justify-center gap-2 w-full">
										<InputDropdown
											onSelect={(value) => setUrlType(value as urlTypeT)}
											value={urlType() || undefined}
											options={[
												{
													label: "Deployments",
													value: "proxyDeployment",
												},
											]}
											class="flex-2 m-0"
											styleVariant="medium"
											placeholder="Type"
										/>
										<div class="flex-10">{urlInput()}</div>
									</div>

									<div class="w-full flex justify-end mt-4">
										<Button
											loading={isCreatingManagedUrl}
											loadingContent={() => <>Creating...</>}
											variant={ButtonVariant.Contained}
										>
											Create
										</Button>
									</div>
								</div>

								<div class="mt-4 bg-secondary-dark p-4 rounded border border-white/5">
									<h4 class="text-white text-sm font-semibold mb-2">Managed URL Instructions</h4>

									<p class="text-gray-400 text-sm space-y-1">
										To configure this Managed URL, please update your DNS settings to point to our
										servers. If you have already updated your DNS settings, please allow some time
										for the changes to propagate.
									</p>

									<Table
										column_grids={["flex-2", "flex-5", "flex-5"]}
										headings={["Type", "Name", "Value"]}
										class="mt-2"
										rows={[
											{
												type: "CNAME",
												name: `${subDomain() || "(subdomain)"}.${domainInfoQuery.data?.name || "your-domain.com"}`,
												target: "ingress.onpatr.cloud",
											},
										]}
										renderRow={(record) => (
											<tr class="table-row text-sm">
												<td class="flex-2 pl-3 flex items-center justify-center">
													<CopyableField
														value={record.type}
														variant={CopyableFieldVariant.Text}
													/>
												</td>
												<td class="flex-5 flex items-center justify-center min-w-0">
													<CopyableField
														value={record.name}
														variant={CopyableFieldVariant.Text}
														innerClass="max-w-full"
													/>
												</td>
												<td class="flex-5 pl-20 flex items-center justify-center min-w-0">
													<CopyableField
														value={record.target}
														variant={CopyableFieldVariant.Text}
														innerClass="max-w-full"
													/>
												</td>
											</tr>
										)}
									/>
								</div>
							</form>

							<div class="flex flex-col gap-2 items-start w-5/5 mt-4">
								<Table
									column_grids={["flex-4", "flex-2", "flex-2", "flex-4"]}
									rows={managedUrlsQuery.data?.urls || []}
									headings={["URL", "Type", "Served by Patr", "Actions"]}
									renderRow={(item) =>
										domainInfoQuery.data && (
											<ManageUrlRow
												domainInfo={domainInfoQuery.data}
												managedUrl={item}
												onUpdate={refetchManagedUrls}
											/>
										)
									}
								/>
							</div>
						</PageContainerBody>
					</Show>
				</ErrorBoundary>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/domains/$id")({
	component: DomainInfo,
});
