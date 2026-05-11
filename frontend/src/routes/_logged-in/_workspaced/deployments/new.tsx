import { createFileRoute } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createSignal, Show } from "solid-js";
import {
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	Input,
	InputType,
	InputDropdown,
	ButtonVariant,
	Button,
	InputLabel,
	ToggleSwitch,
	RangeSlider,
	useToast,
} from "~/components";
import EnvInput from "./-components/env-input";
import {
	Base64String,
	CreateDeploymentRequest,
	CreateDeploymentResponse,
	DeploymentProbe,
	ExposedPortType,
} from "~/bindings";
import PortInput from "./-components/port";
import { createFormAction } from "~/hooks";
import { Uuid } from "~/utils/func";
import { useRunnersQuery, useContainerRegistriesQuery, useContainerTagsQuery } from "~/hooks/fetch";
import { httpRequest } from "~/utils/http-request";
import ProbeInput from "./-components/probe-input";
import ConfigMount from "./-components/config-mount";
import { useNavigate } from "@tanstack/solid-router";

const PATR_REGISTRY = "registry.patr.cloud";

const CreateDeploymentPage = () => {
	const toast = useToast();

	const runnersQuery = useRunnersQuery();
	const repositoriesQuery = useContainerRegistriesQuery(
		() => undefined,
		() => undefined
	);

	const navigate = useNavigate();
	const [name, setName] = createSignal<string>("");
	const [runner, setRunner] = createSignal<string>("");
	const [imageName, setImageName] = createSignal<string>("");
	const [imageTag, setImageTag] = createSignal<string>("");
	const [tagFilter, setTagFilter] = createSignal<string>("");
	const [repositoryId, setRepositoryId] = createSignal<string>("");
	const [configFiles, setConfigFiles] = createSignal<Record<string, Base64String>>({});
	const [startupProbe, setStartupProbe] = createSignal<DeploymentProbe | undefined>(undefined);
	const [minScale, setMinScale] = createSignal(1);
	const [maxScale, setMaxScale] = createSignal(2);
	const [deployOnCreate, setDeployOnCreate] = createSignal(false);
	const [deployOnPush, setDeployOnPush] = createSignal(false);

	const [registry, setRegistry] = createSignal<string>("");
	const [envVars, setEnvVars] = createSignal<Record<string, string>>({});
	const [envValid, setEnvValid] = createSignal(true);

	const [portList, setPortList] = createSignal<Record<string, ExposedPortType>>({});
	const [portsValid, setPortsValid] = createSignal(true);
	const [configMountsValid, setConfigMountsValid] = createSignal(true);

	const isPatrRegistry = () => registry() === PATR_REGISTRY;

	// Debounce tag filter updates to avoid hammering the API on every keystroke
	let tagFilterTimer: ReturnType<typeof setTimeout> | undefined;
	const debouncedSetTagFilter = (value: string) => {
		clearTimeout(tagFilterTimer);
		tagFilterTimer = setTimeout(() => setTagFilter(value), 300);
	};

	const tagsQuery = useContainerTagsQuery(
		() => repositoryId(),
		() => tagFilter()
	);

	const tagSuggestions = () => tagsQuery.data?.tags.map((t) => ({ label: t.tag, value: t.tag })) ?? [];

	const repoSuggestions = () =>
		repositoriesQuery.data?.repositories.map((r) => ({ label: r.name, value: r.id })) ?? [];

	const { onSubmit, isLoading } = createFormAction(async ({ workspaceId }) => {
		if (!envValid() || !portsValid() || !configMountsValid()) {
			toast("Please fix the highlighted errors before submitting", "error");
			return;
		}

		const commonFields = {
			name: name(),
			imageTag: imageTag(),
			runner: runner(),
			machineType: Uuid("b3cf3771-fa39-4281-bfdf-eb2e65a061b6"),
			minHorizontalScale: minScale(),
			maxHorizontalScale: maxScale(),
			environmentVariables: envVars(),
			ports: portList(),
			startupProbe: startupProbe(),
			deployOnCreate: deployOnCreate(),
			deployOnPush: isPatrRegistry() ? deployOnPush() : false,
			configMounts: configFiles(),
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

									<Show
										when={isPatrRegistry()}
										fallback={
											<Input
												class="flex-6"
												placeholder="Image Name"
												type={InputType.Text}
												value={imageName()}
												onInput={(e) => setImageName(e.currentTarget.value)}
											/>
										}
									>
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
									</Show>

									<Show
										when={isPatrRegistry()}
										fallback={
											<Input
												class="flex-2"
												placeholder="Image Tag"
												type={InputType.Text}
												value={imageTag()}
												onInput={(e) => setImageTag(e.currentTarget.value)}
											/>
										}
									>
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
									</Show>
								</div>
							</div>

							<div class="flex gap-8 items-center w-full">
								<InputLabel parentClass="flex-2" for="deployment-runner" label="Runner" />
								<div class="flex-10 flex items-center gap-4 w-full">
									<InputDropdown
										options={
											runnersQuery.data?.runners.map((runner) => ({
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

							<div class="flex gap-8 items-center w-full">
								<InputLabel
									parentClass="flex-2"
									label="Horizontal Scale"
									comments="Min & max replica count"
								/>
								<div class="flex-10">
									<RangeSlider
										min={1}
										max={10}
										valueLow={minScale}
										valueHigh={maxScale}
										onChangeLow={setMinScale}
										onChangeHigh={setMaxScale}
									/>
								</div>
							</div>

							{/* Divider */}
							<div class="border-t border-border-color mt-2" />

							<EnvInput value={() => ({})} onChange={setEnvVars} onValidityChange={setEnvValid} />

							<PortInput value={() => ({})} onChange={setPortList} onValidityChange={setPortsValid} />

							<ProbeInput
								probe={[startupProbe, setStartupProbe]}
								ports={Object.keys(portList()).map((port) => parseInt(port))}
							/>

							<ConfigMount
								value={() => ({})}
								onChange={setConfigFiles}
								onValidityChange={setConfigMountsValid}
							/>

							{/* Divider */}
							<div class="border-t border-border-color mt-2" />

							<div class="flex gap-8 items-center w-full">
								<InputLabel
									parentClass="flex-2"
									label="Deploy on Create"
									comments="Start the deployment immediately after creation"
								/>
								<div class="flex-10">
									<ToggleSwitch checked={deployOnCreate} onChange={setDeployOnCreate} />
								</div>
							</div>

							<Show when={isPatrRegistry()}>
								<div class="flex gap-8 items-center w-full">
									<InputLabel
										parentClass="flex-2"
										label="Deploy on Push"
										comments="Redeploy when a new image is pushed to the registry"
									/>
									<div class="flex-10">
										<ToggleSwitch checked={deployOnPush} onChange={setDeployOnPush} />
									</div>
								</div>
							</Show>
						</div>

						<div class="w-full flex items-end justify-end">
							<Button
								loading={isLoading}
								loadingContent={() => <span>Creating Deployment...</span>}
								type="submit"
								disabled={!envValid() || !portsValid() || !configMountsValid()}
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
