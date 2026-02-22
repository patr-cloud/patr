import { Show } from "solid-js";
import { FiExternalLink } from "solid-icons/fi";
import { GetContainerRepositoryInfoResponse } from "~/bindings";
import { CopyButton, Input, InputLabel, InputType, Link, Tooltip, useToast } from "~/components";
import { formatRelativeTime, formatSize, formatDate, get } from "~/utils/func";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { MaybeAccessor } from "~/utils/types";

interface GeneralInfoProps {
	repositoryInfo: MaybeAccessor<GetContainerRepositoryInfoResponse | undefined>;
}

const General = (props: GeneralInfoProps) => {
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
								<Input
									value={get(props.repositoryInfo)?.repository?.name}
									disabled={true}
									class="flex-10"
									name="repository-name"
									placeholder="Repository Name"
									type={InputType.Text}
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
												? formatRelativeTime(get(props.repositoryInfo)?.repository?.lastUpdated || "")
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
												? formatRelativeTime(get(props.repositoryInfo)?.repository?.created || "")
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
			<div>
				{/* Build and Push New Image */}
				<div>
					<h3 class="text-white text-base font-medium mb-4">Build and Push a New Image</h3>
					<div class="space-y-4">
						{/* Step 1: Login */}
						<div>
							<p class="text-gray-300 text-sm mb-2">1. Login to Patr Registry</p>
							<div class="relative bg-secondary-light rounded px-4 py-3 font-mono text-sm text-gray-300 flex items-center gap-3 group">
								<CopyButton text={`docker login ${registryUrl} -u patr`} />
								<code>docker login {registryUrl} -u patr</code>
							</div>
							<p class="text-gray-300 text-sm mt-2 flex items-center gap-1">
								Use an{" "}
								<Link href="/profile/api-tokens" external={false} class="inline-flex items-center gap-1">
									API token <FiExternalLink size={12} />
								</Link>{" "}
								as the password.
							</p>
						</div>

						{/* Step 2: Build */}
						<div>
							<p class="text-gray-300 text-sm mb-2 ">2. Build your Docker image</p>
							<div class="relative bg-secondary-light rounded px-4 py-3 font-mono text-sm text-gray-300 flex items-center gap-3 group">
								<CopyButton text={`docker build -t ${registryUrl}:<tag> .`} />
								<code>docker build -t {registryUrl}:&lt;tag&gt; .</code>
							</div>
						</div>

						{/* Step 3: Push */}
						<div>
							<p class="text-gray-300 text-sm mb-2">3. Push the image</p>
							<div class="relative bg-secondary-light rounded px-4 py-3 font-mono text-sm text-gray-300 flex items-center gap-3 group">
								<CopyButton text={`docker push ${registryUrl}:<tag>`} />
								<code>docker push {registryUrl}:&lt;tag&gt;</code>
							</div>
						</div>
					</div>
				</div>

				{/* Divider */}
				<div class="border-t border-border-color mt-4 mb-3" />

				{/* Push Existing Image */}
				<div>
					<h3 class="text-white text-base font-medium mb-4">Push an Existing Image</h3>
					<div class="space-y-4">
						{/* Step 1: Login */}
						<div>
							<p class="text-gray-300 text-sm mb-2">1. Login to Patr Registry</p>
							<div class="relative bg-secondary-light rounded px-4 py-3 font-mono text-sm text-gray-300 flex items-center gap-3 group">
								<CopyButton text={`docker login ${registryUrl} -u patr`} />
								<code>docker login {registryUrl} -u patr</code>
							</div>
							<p class="text-gray-300 text-sm mt-2 flex items-center gap-1">
								Use your Patr{" "}
								<Link href="/profile/api-tokens" external={false} class="inline-flex items-center gap-1">
									API token <FiExternalLink size={12} />
								</Link>{" "}
								as the password.
							</p>
						</div>

						{/* Step 2: Tag */}
						<div>
							<p class="text-gray-300 text-sm mb-2">2. Tag the existing image</p>
							<div class="relative bg-secondary-light rounded px-4 py-3 font-mono text-sm text-gray-300 flex items-center gap-3 group">
								<CopyButton text={`docker tag <existing-image>:<tag> ${registryUrl}:<tag>`} />
								<code>docker tag &lt;existing-image&gt;:&lt;tag&gt; {registryUrl}:&lt;tag&gt;</code>
							</div>
						</div>

						{/* Step 3: Push */}
						<div>
							<p class="text-gray-300 text-sm mb-2">3. Push the image</p>
							<div class="relative bg-secondary-light rounded px-4 py-3 font-mono text-sm text-gray-300 flex items-center gap-3 group">
								<CopyButton text={`docker push ${registryUrl}:<tag>`} />
								<code>docker push {registryUrl}:&lt;tag&gt;</code>
							</div>
						</div>
					</div>
				</div>
			</div>
		</div>
	);
};
