import { createSignal, Show } from "solid-js";
import { FiChevronRight, FiExternalLink } from "solid-icons/fi";
import { GetContainerRepositoryInfoResponse } from "~/bindings";
import { CopyableField, CopyableFieldVariant, Input, InputType, InputWithLabel, Link, Tooltip } from "~/components";
import { formatRelativeTime, formatSize, formatDate, get } from "~/utils/func";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { MaybeAccessor } from "~/utils/types";
import { PullCommand } from "./registry-ui";

interface GeneralInfoProps {
	repositoryInfo: MaybeAccessor<GetContainerRepositoryInfoResponse | undefined>;
}

const General = (props: GeneralInfoProps) => {
	const [workspaceId] = useLastWorkspaceId();

	return (
		<div class="w-full">
			<Show when={get(props.repositoryInfo)} fallback={<div class="text-gray-400 p-6">Loading...</div>}>
				<div>
					{/* Repository Details */}
					<div>
						<div class="space-y-4">
							<InputWithLabel for="repository-name" label="Repository Name">
								<CopyableField
									variant={CopyableFieldVariant.Input}
									value={`registry.patr.cloud/${workspaceId()}/${get(props.repositoryInfo)?.repository?.name}`}
									buttonPosition="start"
								/>
							</InputWithLabel>

							<InputWithLabel for="repository-size" label="Size">
								<Input
									value={formatSize(get(props.repositoryInfo)?.repository?.size)}
									disabled={true}
									name="repository-size"
									placeholder="Repository Size"
									type={InputType.Text}
								/>
							</InputWithLabel>

							<InputWithLabel for="repository-last-updated" label="Last Updated">
								<Tooltip
									content={formatDate(get(props.repositoryInfo)?.repository?.lastUpdated || "")}
									class="text-white"
								>
									<Input
										value={
											get(props.repositoryInfo)?.repository?.lastUpdated
												? formatRelativeTime(
														get(props.repositoryInfo)?.repository?.lastUpdated || ""
													)
												: "N/A"
										}
										disabled={true}
										name="repository-last-updated"
										placeholder="Repository last updated"
										type={InputType.Text}
									/>
								</Tooltip>
							</InputWithLabel>

							<InputWithLabel for="repository-created" label="Created">
								<Tooltip
									content={formatDate(get(props.repositoryInfo)?.repository?.created || "")}
									class="text-white"
								>
									<Input
										value={
											get(props.repositoryInfo)?.repository?.created
												? formatRelativeTime(
														get(props.repositoryInfo)?.repository?.created || ""
													)
												: "N/A"
										}
										disabled={true}
										name="repository-created"
										placeholder="Repository created"
										type={InputType.Text}
									/>
								</Tooltip>
							</InputWithLabel>
						</div>
					</div>

					<div class="py-6">
						<PullCommand
							label="Pull this image"
							reference={`registry.patr.cloud/${workspaceId()}/${
								get(props.repositoryInfo)?.repository?.name
							}:latest`}
						/>
					</div>

					<CollapsiblePushInstructions repositoryName={get(props.repositoryInfo)?.repository?.name} />
				</div>
			</Show>
		</div>
	);
};

export default General;

const CollapsiblePushInstructions = (props: { repositoryName: string | undefined }) => {
	const [open, setOpen] = createSignal(false);

	return (
		<div class="border-t border-border-color pt-4">
			<button
				type="button"
				onClick={() => setOpen((value) => !value)}
				class="flex items-center gap-2 text-sm text-gray-300 hover:text-white"
			>
				<FiChevronRight size={16} class={`transition-transform ${open() ? "rotate-90" : ""}`} />
				How to push a new image
			</button>
			<Show when={open()}>
				<PushInstructions repositoryName={props.repositoryName} />
			</Show>
		</div>
	);
};

const PushInstructions = (props: { repositoryName: string | undefined }) => {
	const [workspaceId] = useLastWorkspaceId();

	const registryHost = "registry.patr.cloud";
	const registryUrl = () => `${registryHost}/${workspaceId()}/${props.repositoryName || "<repository-name>"}`;
	return (
		<div class="py-8">
			<h2 class="text-white text-lg font-semibold mb-4">Push Instructions</h2>
			<div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
				{/* Build and Push New Image */}
				<div class="border border-border-color rounded-xs p-5">
					<h3 class="text-white text-base font-medium mb-4">Build and Push a New Image</h3>
					<div class="space-y-4">
						{/* Step 1: Login */}
						<div>
							<p class="text-gray-300 text-sm mb-2">1. Login to Patr Registry</p>
							<CopyableField value={`docker login ${registryHost} -u patr`} innerClass="font-mono" />
							<p class="text-gray-300 text-sm mt-2 flex items-center gap-1">
								Use an{" "}
								<Link
									href="/profile/api-tokens"
									external={false}
									class="inline-flex items-center gap-1"
								>
									API token <FiExternalLink size={12} />
								</Link>{" "}
								as the password.
							</p>
						</div>

						{/* Step 2: Build */}
						<div>
							<p class="text-gray-300 text-sm mb-2">2. Build your Docker image</p>
							<CopyableField value={`docker build -t ${registryUrl()}:<tag> .`} innerClass="font-mono" />
						</div>

						{/* Step 3: Push */}
						<div>
							<p class="text-gray-300 text-sm mb-2">3. Push the image</p>
							<CopyableField value={`docker push ${registryUrl()}:<tag>`} innerClass="font-mono" />
						</div>
					</div>
				</div>

				{/* Push Existing Image */}
				<div class="border border-border-color rounded-xs p-5">
					<h3 class="text-white text-base font-medium mb-4">Push an Existing Image</h3>
					<div class="space-y-4">
						{/* Step 1: Login */}
						<div>
							<p class="text-gray-300 text-sm mb-2">1. Login to Patr Registry</p>
							<CopyableField value={`docker login ${registryHost} -u patr`} innerClass="font-mono" />
							<p class="text-gray-300 text-sm mt-2 flex items-center gap-1">
								Use an&nbsp;
								<Link
									href="/profile/api-tokens"
									external={false}
									class="inline-flex items-center gap-1"
								>
									API token <FiExternalLink size={12} />
								</Link>
								&nbsp; as the password.
							</p>
						</div>

						{/* Step 2: Tag */}
						<div>
							<p class="text-gray-300 text-sm mb-2">2. Tag the existing image</p>
							<CopyableField
								value={`docker tag <existing-image>:<tag> ${registryUrl()}:<tag>`}
								innerClass="font-mono"
							/>
						</div>

						{/* Step 3: Push */}
						<div>
							<p class="text-gray-300 text-sm mb-2">3. Push the image</p>
							<CopyableField value={`docker push ${registryUrl()}:<tag>`} innerClass="font-mono" />
						</div>
					</div>
				</div>
			</div>
		</div>
	);
};
