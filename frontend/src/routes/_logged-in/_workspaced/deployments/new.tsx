import { createFileRoute } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createMemo, createResource, createSignal } from "solid-js";
import {
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	InputDropdown,
	ButtonVariant,
	Button,
} from "~/components";
import Input, { InputType } from "~/components/input";
import InputLabel from "~/components/input-label";
import EnvInput from "./-components/env-input";
import {
	Base64String,
	CreateDeploymentRequest,
	CreateDeploymentResponse,
	DeploymentProbe,
	EnvironmentVariableValue,
	ExposedPortType,
	ListContainerRepositoriesResponse,
	ListContainerRepositoryTagsResponse,
	ListRunnersForWorkspaceResponse,
} from "~/bindings";
import PortInput from "./-components/port";
import { createFormAction, useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import { convertFileToBase64, Uuid } from "~/utils/func";
import { useToast } from "~/components";
import ProbeInput from "./-components/probe-input";
import ConfigMount, { ConfigMountT } from "./-components/config-mount";
import { useNavigate } from "@tanstack/solid-router";

const PATR_REGISTRY = "registry.patr.cloud";

const CreateDeploymentPage = () => {
	const [authState] = useAuthState();
	const [lastUsedWorkspaceId] = useLastWorkspaceId();
	const toast = useToast();

	const fetchParams = createMemo(() => {
		return [authState(), lastUsedWorkspaceId()] as const;
	});

	const [runners] = createResource(fetchParams, async ([auth, wsId]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn") {
			return { runners: [] };
		}
		const response = await httpRequest<ListRunnersForWorkspaceResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/runner`,
			{
				method: "GET",
			}
		);

		if (!response.ok) {
			console.error("Failed to fetch runners:", response.data.error);
			toast("Failed to fetch runners", "error");
			return { runners: [] };
		}

		return response.data;
	});

	const [repositories] = createResource(fetchParams, async ([auth, wsId]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn") {
			return { repositories: [] };
		}
		const response = await httpRequest<ListContainerRepositoriesResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry`,
			{
				method: "GET",
			}
		);
		if (!response.ok) {
			console.error("Failed to fetch repositories:", response.data.error);
			return { repositories: [] };
		}
		return response.data;
	});

	const navigate = useNavigate();
	const [name, setName] = createSignal<string>("");
	const [runner, setRunner] = createSignal<string>("");
	const [imageName, setImageName] = createSignal<string>("");
	const [imageTag, setImageTag] = createSignal<string>("");
	const [tagFilter, setTagFilter] = createSignal<string>("");
	const [repositoryId, setRepositoryId] = createSignal<string>("");
	const [configFiles, setConfigFiles] = createSignal<ConfigMountT>({});
	const [startupProbe, setStartupProbe] = createSignal<DeploymentProbe | undefined>(undefined);

	const [registry, setRegistry] = createSignal<string>("");
	const [envList, setEnvList] = createSignal<{ key: string; value: EnvironmentVariableValue }[]>([]);

	const [portList, setPortList] = createSignal<{
		[key: string]: ExposedPortType;
	}>({});

	const isPatrRegistry = () => registry() === PATR_REGISTRY;

	// Debounce tag filter updates to avoid hammering the API on every keystroke
	let tagFilterTimer: ReturnType<typeof setTimeout> | undefined;
	const debouncedSetTagFilter = (value: string) => {
		clearTimeout(tagFilterTimer);
		tagFilterTimer = setTimeout(() => setTagFilter(value), 300);
	};

	const tagFetchParams = createMemo(() => {
		return [authState(), lastUsedWorkspaceId(), repositoryId(), tagFilter()] as const;
	});

	const [repositoryTags] = createResource(tagFetchParams, async ([auth, wsId, repoId, tagSearch]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn" || !repoId) {
			return { tags: [] };
		}
		const url = new URL(`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry/${repoId}/tag`);
		if (tagSearch) url.searchParams.set("tag", tagSearch);
		const response = await httpRequest<ListContainerRepositoryTagsResponse>(url.toString(), {
			method: "GET",
		});
		if (!response.ok) {
			console.error("Failed to fetch tags:", response.data.error);
			return { tags: [] };
		}
		return response.data;
	});

	const tagSuggestions = () => repositoryTags()?.tags.map((t) => ({ label: t.tag, value: t.tag })) ?? [];

	const repoSuggestions = () => repositories()?.repositories.map((r) => ({ label: r.name, value: r.id })) ?? [];

	const { onSubmit, isLoading } = createFormAction(async ({ workspaceId }) => {
		let configMounts: Record<string, Base64String> = {};
		for (const [key, file] of Object.entries(configFiles())) {
			const byteArray = await convertFileToBase64(file);
			configMounts[key] = byteArray;
		}

		const commonFields = {
			name: name(),
			imageTag: imageTag(),
			runner: runner(),
			machineType: Uuid("b3cf3771-fa39-4281-bfdf-eb2e65a061b6"),
			minHorizontalScale: 1,
			maxHorizontalScale: 2,
			environmentVariables: Object.fromEntries(envList().map((env) => [env.key, env.value])),
			ports: portList(),
			deployOnCreate: false,
			deployOnPush: false,
			configMounts,
		};

		const requestBody = (
			isPatrRegistry()
				? { ...commonFields, registry: PATR_REGISTRY, repositoryId: repositoryId() }
				: { ...commonFields, registry: registry(), imageName: imageName() }
		) as CreateDeploymentRequest;

		const response = await httpRequest<CreateDeploymentResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/deployment`,
			{
				method: "POST",
				body: JSON.stringify(requestBody),
			}
		);

		if (!response.ok) {
			console.error("Failed to create deployment:", response.data.error);
			toast("Failed to create deployment", "error");
			return;
		}

		toast("Deployment created successfully", "success");

		navigate({ to: `/deployments/${response.data.id}` });
		console.log("Deployment created:", response.data);
	});

	return (
		<>
			<Title>New Deployment | Patr</Title>
			<PageContainer>
				<PageContainerHead
					breadcrumbs={[
						{
							label: "Deployments",
							url: "/deployments",
						},
						{
							label: "New",
						},
					]}
					subText="A deployment represents a containerized application running on a runner."
				/>
				<PageContainerBody class="flex flex-col justify-between gap-8">
					<form onSubmit={onSubmit} class="flex flex-col gap-6 justify-between w-full flex-1">
						<div class="flex flex-col gap-5  items-start w-full">
							<div class="flex gap-8 items-center w-full">
								<InputLabel parentClass="flex-2" for="deployment-name" label="Name" />
								<Input
									value={name()}
									onInput={(e) => {
										e.preventDefault();
										setName(e.currentTarget.value);
									}}
									class="flex-10"
									name="deployment-name"
									placeholder="Enter Deployment Name"
									type={InputType.Text}
								/>
							</div>

							<div class="flex gap-8 items-center w-full">
								<InputLabel parentClass="flex-2" for="deployment-registry" label="Image" />
								<div class="flex-10 flex items-center gap-4 w-full">
									<InputDropdown
										options={[
											{ value: PATR_REGISTRY, label: "Patr Registry" },
											{ value: "docker.io", label: "Docker Hub" },
										]}
										value={registry()}
										onSelect={(val) => {
											setRegistry(val);
											setRepositoryId("");
											setImageName("");
											setImageTag("");
											setTagFilter("");
										}}
										class="flex-4"
										name="deployment-registry"
										placeholder="Select Registry"
									/>

									{isPatrRegistry() ? (
										<Input
											class="flex-6"
											placeholder="Select Repository"
											suggestions={repoSuggestions()}
											allowCustomValue={false}
											value={repositoryId()}
											onSelect={(id) => {
												setRepositoryId(id);
												setImageTag("");
												setTagFilter("");
											}}
										/>
									) : (
										<Input
											class="flex-6"
											placeholder="Image Name"
											type={InputType.Text}
											value={imageName()}
											onInput={(e) => setImageName(e.currentTarget.value)}
										/>
									)}

									{isPatrRegistry() ? (
										<Input
											class="flex-2"
											placeholder="Image Tag"
											value={imageTag()}
											suggestions={tagSuggestions()}
											allowCustomValue={true}
											onInput={(e) => {
												setImageTag(e.currentTarget.value);
												debouncedSetTagFilter(e.currentTarget.value);
											}}
											onSelect={setImageTag}
										/>
									) : (
										<Input
											class="flex-2"
											placeholder="Image Tag"
											type={InputType.Text}
											value={imageTag()}
											onInput={(e) => setImageTag(e.currentTarget.value)}
										/>
									)}
								</div>
							</div>

							<div class="flex gap-8 items-center w-full">
								<InputLabel parentClass="flex-2" for="deployment-runner" label="Runner" />
								<div class="flex-10 flex items-center gap-4 w-full">
									<InputDropdown
										options={
											runners()?.runners.map((runner) => ({
												value: runner.id,
												label: runner.name,
											})) ?? []
										}
										value={runner()}
										onSelect={setRunner}
										class="flex-4"
										name="deployment-runner"
										placeholder="Select Runner"
									/>
								</div>
							</div>

							<EnvInput
								envList={envList}
								onAdd={(key, value) => {
									setEnvList((prev) => [...prev, { key, value }]);
								}}
								onDelete={(key) => {
									setEnvList((prev) => prev.filter((env) => env.key !== key));
								}}
							/>

							<PortInput
								onAdd={(key, value) => {
									setPortList((prev) => ({ ...prev, [key]: value }));
								}}
								onDelete={(key) => {
									setPortList((prev) => {
										const newPorts = { ...prev };
										delete newPorts[key];
										return newPorts;
									});
								}}
								portList={portList}
							/>

							<ProbeInput
								probe={[startupProbe, setStartupProbe]}
								ports={Object.keys(portList()).map((port) => parseInt(port))}
							/>

							<ConfigMount selectedFiles={configFiles} setSelectedFiles={setConfigFiles} />
						</div>

						<div class="w-full flex items-end justify-end">
							<Button
								loading={isLoading}
								loadingContent={() => <span>Creating Deployment...</span>}
								type="submit"
								variant={ButtonVariant.Contained}
							>
								Create
							</Button>
						</div>
					</form>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/deployments/new")({
	component: CreateDeploymentPage,
});
