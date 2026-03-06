import { useNavigate, useParams } from "@solidjs/router";
import { createMemo, createResource, createSignal, ErrorBoundary, Suspense } from "solid-js";
import {
	CreateManagedURLRequest,
	CreateManagedURLResponse,
	DeleteDomainInWorkspaceResponse,
	GetDomainInfoInWorkspaceResponse,
	ListManagedURLResponse,
} from "~/bindings";
import {
	Button,
	ButtonVariant,
	CopyButton,
	DeleteModal,
	Input,
	InputDropdown,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	Table,
	useToast,
} from "~/components";
import { createAuthenticatedAction, createFormAction, useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import DeploymentOption from "./deployment-option";
import ManageUrlRow from "./managed-url-component";

type urlTypeT = "proxyUrl" | "redirect" | "proxyDeployment" | "proxyStaticSite";

const DomainInfo = () => {
	const params = useParams();

	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const navigate = useNavigate();

	const [subDomain, setSubDomain] = createSignal("");
	const [path, setPath] = createSignal("");
	const [urlType, setUrlType] = createSignal<urlTypeT | null>(null);
	const [target, setTarget] = createSignal<string | null>(null);
	const [deploymentPort, setDeploymentPort] = createSignal<number | null>(null);

	const resourceParams = createMemo(() => {
		return [authState(), workspaceId(), params.id] as const;
	});

	const [domainInfo, { refetch: refetchDomainInfo }] = createResource(
		resourceParams,
		async ([auth, wsId, domainId]) => {
			if (!wsId || !auth || auth.type !== "LoggedIn" || !domainId) {
				return;
			}

			// Fetch domain info logic goes here
			const resource = await httpRequest<GetDomainInfoInWorkspaceResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/domain/${domainId}`,
				{
					method: "GET",
					headers: {
						"Content-Type": "application/json",
					},
				}
			);

			if (!resource.ok) {
				console.error("Failed to fetch domain info:", resource.data.error);
				toast("Failed to fetch domain info", "error");
				return;
			}

			return resource.data;
		}
	);

	const resourceParamsManagedUrls = createMemo(() => {
		return [authState(), workspaceId(), params.id] as const;
	});
	const [managedUrls, { refetch: refetchManagedUrls }] = createResource(
		resourceParamsManagedUrls,
		async ([auth, wsId, domainId]) => {
			if (!wsId || !auth || auth.type !== "LoggedIn" || !domainId) {
				return;
			}

			// Fetch managed URLs logic goes here
			const resource = await httpRequest<ListManagedURLResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/infrastructure/managed-url?domainId=${domainId}`,
				{
					method: "GET",
					headers: {
						"Content-Type": "application/json",
					},
				}
			);

			if (!resource.ok) {
				console.error("Failed to fetch managed URLs:", resource.data.error);
				toast("Failed to fetch managed URLs", "error");
				return;
			}

			return resource.data;
		}
	);

	const { onSubmit: onSubmitCreateManagedUrl, isLoading: isCreatingManagedUrl } = createFormAction(
		async ({ workspaceId }) => {
			const domainId = params.id;

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

			const requestBody: CreateManagedURLRequest = {
				domainId,
				subDomain: subDomain(),
				path: path(),
				type: "proxyDeployment",
				deploymentId: targetVal,
				port: deploymentPort() || 80,
			};

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

			refetchManagedUrls();
		}
	);

	const { execute: onVerifyClick, isLoading: verifyLoading } = createAuthenticatedAction(async ({ workspaceId }) => {
		const domainId = params.id;

		if (!domainId) {
			toast("Unable to verify domain", "error");
			return;
		}

		const verifyResp = await httpRequest<GetDomainInfoInWorkspaceResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/domain/${domainId}/verify`,
			{
				method: "POST",
			}
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
			const domainId = params.id;

			if (!workspaceId || !accessToken || !domainId) {
				toast("Unable to delete domain", "error");
				return;
			}

			const deleteDomainResp = await httpRequest<DeleteDomainInWorkspaceResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/domain/${domainId}`,
				{
					method: "DELETE",
				}
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
			navigate("/domains");
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
		<PageContainer>
			<ErrorBoundary
				fallback={(err, reset) => (
					<div>
						<p>Error loading runners: {err.message}</p>
						<button onClick={reset}>Retry</button>
					</div>
				)}
			>
				<Suspense fallback={<div class="text-white">Loading...</div>}>
					<PageContainerHead
						breadcrumbs={[
							{
								label: "Domains",
								url: "/domains",
							},
							{
								label: domainInfo.latest?.name || "Loading...",
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
									resourceName={domainInfo.latest?.name || ""}
								/>
								{!domainInfo.latest?.isVerified ? (
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
									<Input disabled={true} value={domainInfo.latest?.name} class="flex-2" placeholder="Domain" />
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
									To configure this Managed URL, please update your DNS settings to point to our servers. If you have
									already updated your DNS settings, please allow some time for the changes to propagate.
								</p>

								<Table
									column_grids={["flex-2", "flex-4", "flex-4"]}
									headings={["Type", "Name", "Value"]}
									class="mt-2"
									rows={[
										{
											type: "CNAME",
											name: `${subDomain() || "(subdomain)"}.${domainInfo.latest?.name || "your-domain.com"}`,
											target: "ingress.patr.cloud",
										},
									]}
									renderRow={(record) => (
										<tr class="table-row text-sm">
											<td class="flex-2 pl-3 flex items-center justify-center">
												<span class="truncate">{record.type}</span>
												<CopyButton text={record.type} />
											</td>
											<td class="flex-4 flex items-center justify-center min-w-0">
												<span class="truncate max-w-full">{record.name}</span>
												<CopyButton text={record.name} />
											</td>
											<td class="flex-4 pl-20 flex items-center justify-center min-w-0">
												<span class="truncate max-w-full">{record.target}</span>
												<CopyButton text={record.target} />
											</td>
										</tr>
									)}
								/>
							</div>
						</form>

						<div class="flex flex-col gap-2 items-start w-5/5 mt-4">
							<Table
								column_grids={["flex-3", "flex-3", "flex-3"]}
								rows={managedUrls.latest?.urls || []}
								headings={["Domain ID", "Domain Name", "Actions"]}
								renderRow={(item) =>
									domainInfo.latest && (
										<ManageUrlRow domainInfo={domainInfo.latest} managedUrl={item} onUpdate={refetchManagedUrls} />
									)
								}
							/>
						</div>
					</PageContainerBody>
				</Suspense>
			</ErrorBoundary>
		</PageContainer>
	);
};

export default DomainInfo;
