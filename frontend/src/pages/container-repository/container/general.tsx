import { Show } from "solid-js";
import { FiExternalLink } from "solid-icons/fi";
import { GetContainerRepositoryInfoResponse } from "~/bindings";
import { CopyableField, CopyableFieldVariant, Input, InputLabel, InputType, Link, Tooltip } from "~/components";
import { formatRelativeTime, formatSize, formatDate, get } from "~/utils/func";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { MaybeAccessor } from "~/utils/types";

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
							{/* Repo Name */}
							<div class="flex items-center gap-4">
								<InputLabel parentClass="flex-2" for="repository-name" label="Repository Name" />
								<CopyableField
									variant={CopyableFieldVariant.Input}
									value={`registry.patr.cloud/${workspaceId()}/${get(props.repositoryInfo)?.repository?.name}`}
									buttonPosition="start"
									class="flex-10"
								/>
							</div>

							{/* Size */}
							<div class="flex items-center gap-4">
								<InputLabel parentClass="flex-2" for="repository-size" label="Size" />
								<Input
									value={formatSize(get(props.repositoryInfo)?.repository?.size)}
									disabled={true}
									class="flex-10"
									name="repository-size"
									placeholder="Repository Size"
									type={InputType.Text}
								/>
							</div>

							{/* Last Updated */}
							<div class="flex items-center gap-4">
								<InputLabel parentClass="flex-2" for="repository-last-updated" label="Last Updated" />
								<Tooltip
									content={formatDate(get(props.repositoryInfo)?.repository?.lastUpdated || "")}
									class="flex-10 text-white"
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
										class="flex-10"
										name="repository-last-updated"
										placeholder="Repository last updated"
										type={InputType.Text}
									/>
								</Tooltip>
							</div>
							<div class="flex items-center gap-4">
								<InputLabel parentClass="flex-2" for="repository-created" label="Created" />
								<Tooltip
									content={formatDate(get(props.repositoryInfo)?.repository?.created || "")}
									class="flex-10 text-white"
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
										class="flex-10"
										name="repository-created"
										placeholder="Repository created"
										type={InputType.Text}
									/>
								</Tooltip>
							</div>
						</div>
					</div>
					<PushInstructions repositoryName={get(props.repositoryInfo)?.repository?.name} />
				</div>
			</Show>
		</div>
	);
};

export default General;

const PushInstructions = (props: { repositoryName: string | undefined }) => {
	const [workspaceId] = useLastWorkspaceId();

	const registryUrl = `registry.patr.cloud/${workspaceId()}/${props.repositoryName || "<repository-name>"}`;
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
							<CopyableField value={`docker login ${registryUrl} -u patr`} innerClass="font-mono" />
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
							<CopyableField value={`docker build -t ${registryUrl}:<tag> .`} innerClass="font-mono" />
						</div>

						{/* Step 3: Push */}
						<div>
							<p class="text-gray-300 text-sm mb-2">3. Push the image</p>
							<CopyableField value={`docker push ${registryUrl}:<tag>`} innerClass="font-mono" />
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
							<CopyableField value={`docker login ${registryUrl} -u patr`} innerClass="font-mono" />
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
								value={`docker tag <existing-image>:<tag> ${registryUrl}:<tag>`}
								innerClass="font-mono"
							/>
						</div>

						{/* Step 3: Push */}
						<div>
							<p class="text-gray-300 text-sm mb-2">3. Push the image</p>
							<CopyableField value={`docker push ${registryUrl}:<tag>`} innerClass="font-mono" />
						</div>
					</div>
				</div>
			</div>
		</div>
	);
};
