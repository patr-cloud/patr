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
	DeleteModal,
	Input,
	InputDropdown,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	Table,
	useToast,
} from "~/components";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import { EventT } from "~/utils/types";
import DeploymentOption from "./deployment-option";
import ManageUrlRow from "./managed-url-component";
import useIsAllowed, { useResourceIdPermissions } from "~/hooks/use-fetch/use-allowed";

type urlTypeT = "proxyUrl" | "redirect" | "proxyDeployment" | "proxyStaticSite";

const DomainInfo = () => {
	const params = useParams();

	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const navigate = useNavigate();
	const [isVerifying, setIsVerifying] = createSignal(false);

	const [subDomain, setSubDomain] = createSignal("");
	const [path, setPath] = createSignal("");
	const [urlType, setUrlType] = createSignal<urlTypeT | null>(null);
	const [target, setTarget] = createSignal<string | null>(null);
	const [deploymentPort, setDeploymentPort] = createSignal<number | null>(null);

	const permissions = useResourceIdPermissions("domain", () => params.id!);
	const [isAllowedCreateManagedUrl] = useIsAllowed("managedUrl", "create");

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

	const onSubmitCreateManagedUrl = async (e: EventT<SubmitEvent, HTMLFormElement>) => {
		e.preventDefault();
		// Create managed URL logic goes here

		const auth = authState();
		const wsId = workspaceId();
		const domainId = params.id;

		if (!domainId || !wsId || !auth || auth.type !== "LoggedIn") {
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
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/infrastructure/managed-url`,
			{
				method: "POST",
				body: JSON.stringify(requestBody),
				headers: {
					"Content-Type": "application/json",
				},
			}
		);

		if (!response.ok) {
			console.error("Failed to create managed URL:", response.data.error);
			toast("Failed to create managed URL", "error");
			return;
		}

		toast("Managed URL created successfully", "success");

		refetchManagedUrls();
	};

	const onVerifyClick = async (e: EventT<MouseEvent, HTMLButtonElement>) => {
		if (!permissions().verify) {
			toast("You do not have permission to verify this domain", "error");
			return;
		}

		// Verify domain logic goes here
		const auth = authState();
		const wsId = workspaceId();
		const domainId = params.id;

		if (!wsId || !auth || auth.type !== "LoggedIn" || !domainId) {
			toast("Unable to verify domain", "error");
			return;
		}

		const verifyResp = await httpRequest<GetDomainInfoInWorkspaceResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/domain/${domainId}/verify`,
			{
				method: "POST",
				headers: {
					"Content-Type": "application/json",
				},
			}
		);

		if (!verifyResp.ok) {
			console.error("Failed to verify domain:", verifyResp.data.error);
			toast("Failed to verify domain", "error");
			return;
		}

		toast("Domain verification initiated", "success");
		refetchDomainInfo();

		setIsVerifying(true);
	};

	const onClickDelete = async (e: EventT<MouseEvent, HTMLButtonElement>) => {
		e.preventDefault();

		// Delete domain logic goes here
		const auth = authState();
		const wsId = workspaceId();
		const domainId = params.id;

		if (!wsId || !auth || auth.type !== "LoggedIn" || !domainId) {
			toast("Unable to delete domain", "error");
			return;
		}

		const url = `${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/domain/${domainId}`;
		console.log("Deleting domain with url:", url);
		const response = await fetch(url, {
			method: "DELETE",
			headers: {
				"Content-Type": "application/json",
			},
		});

		const deleteDomainResp = await response.json();

		console.log(deleteDomainResp);

		if (!response.ok) {
			console.error("Failed to delete domain:", deleteDomainResp.error);
			if (deleteDomainResp.error === "resourceInUse") {
				toast("Cannot delete domain: Domain is in use by managed URL(s)", "error");
				return;
			}
			toast("Failed to delete domain", "error");
			return;
		}

		toast("Domain deleted successfully", "success");
		// navigate("/domains");
	};

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
						title="Domains"
						titleUrl="/domains"
						subTitle={domainInfo.latest?.name}
						actions={() => (
							<div class="flex items-center justify-center gap-2">
								{true && (
									<DeleteModal
										title="Delete Domain"
										onClickDelete={onClickDelete}
										resourceName={domainInfo.latest?.name || ""}
									/>
								)}
								{true && !domainInfo.latest?.isVerified ? (
									<Button
										type="button"
										onClick={onVerifyClick}
										variant={ButtonVariant.Contained}
										disabled={isVerifying()}
									>
										{isVerifying() ? "Verifying..." : "Verify"}
									</Button>
								) : undefined}
							</div>
						)}
					/>
					<PageContainerBody>
						{true && (
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
										<Button variant={ButtonVariant.Contained}>Create</Button>
									</div>
								</div>
							</form>
						)}

						<div class="flex flex-col gap-2 items-start w-5/5 mt-4">
							<Table
								column_grids={["flex-3", "flex-3", "flex-[0.3]"]}
								rows={managedUrls.latest?.urls || []}
								headings={["Domain ID", "Domain Name", " "]}
								renderRow={(item) =>
									domainInfo.latest && (
										<ManageUrlRow domainInfo={domainInfo.latest} managedUrl={item} onUpdate={refetchManagedUrls} />
									)
								}
							/>
							{/* {domainInfo.latest && managedUrls.latest?.urls.at(0) && (
                <ManagedUrlComponent
                  domainInfo={domainInfo.latest}
                  managedUrl={managedUrls.latest?.urls[0]!}
                  onUpdate={refetchManagedUrls}
                />
              )} */}
						</div>
					</PageContainerBody>
				</Suspense>
			</ErrorBoundary>
		</PageContainer>
	);
};

export default DomainInfo;
