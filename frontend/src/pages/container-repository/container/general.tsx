import { Show } from "solid-js";
import { FiCopy, FiExternalLink } from "solid-icons/fi";
import { GetContainerRepositoryInfoResponse } from "~/bindings";
import { Input, InputLabel, InputType, Link, Tooltip, useToast } from "~/components";
import { formatRelativeTime, formatSize, parseDate } from "~/utils/func";
import { useLastWorkspaceId } from "~/hooks/state-hooks";

interface GeneralInfoProps {
	repositoryInfo: GetContainerRepositoryInfoResponse | undefined;
}

const General = (props: GeneralInfoProps) => {
	const toast = useToast();
	const [workspaceId] = useLastWorkspaceId();
	const handleCopy = async (text: string) => {
		try {
			await navigator.clipboard.writeText(text);
			toast("Copied to clipboard", "success");
		} catch (error) {
			console.error("Failed to copy:", error);
			toast("Failed to copy", "error");
		}
	};

	const registryUrl = "registry.patr.cloud";
	const repositoryPath = () =>
		`${registryUrl}/${workspaceId()}/${props.repositoryInfo?.repository?.name || "<repository-name>"}`;

	return (
		<div class="w-full">
			<Show when={props.repositoryInfo} fallback={<div class="text-gray-400 p-6">Loading...</div>}>
				<div>
					{/* Repository Details */}
					<div>
						<div class="space-y-4">
							{/* Repo Name */}
							<div class="flex items-center gap-4">
								<InputLabel parentClass="flex-2" for="repository-name" label="Repository Name" />
								<Input
									value={props.repositoryInfo?.repository?.name}
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
									value={formatSize(props.repositoryInfo?.repository?.size)}
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
									content={
										parseDate(props.repositoryInfo?.repository?.lastUpdated || "")?.toLocaleString("en-US", {
											year: "numeric",
											month: "short",
											day: "numeric",
											hour: "2-digit",
											minute: "2-digit",
											second: "2-digit",
										}) || "N/A"
									}
									position="top"
									class="flex-10 text-white"
								>
									<Input
										value={
											props.repositoryInfo?.repository?.lastUpdated
												? formatRelativeTime(props.repositoryInfo.repository.lastUpdated)
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
									content={
										parseDate(props.repositoryInfo?.repository?.created || "")?.toLocaleString("en-US", {
											year: "numeric",
											month: "short",
											day: "numeric",
											hour: "2-digit",
											minute: "2-digit",
											second: "2-digit",
										}) || "N/A"
									}
									position="top"
									class="flex-10 text-white"
								>
									<Input
										value={
											props.repositoryInfo?.repository?.created
												? formatRelativeTime(props.repositoryInfo.repository.created)
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

					{/* Push Instructions */}
					<div class="py-8">
						<h2 class="text-white text-lg font-semibold mb-4">Push Instructions</h2>
						<div>
							{/* Build and Push New Image */}
							<div>
								<h3 class="text-white text-base font-medium mb-4">Build and Push a New Image</h3>
								<div class="space-y-4">
									{/* Step 1: Build */}
									<div>
										<p class="text-gray-300 text-sm mb-2 ">1. Build your Docker image</p>
										<div class="relative bg-secondary-light rounded px-4 py-3 font-mono text-sm text-gray-300 flex items-center gap-3 group">
											<button
												onClick={() => handleCopy("docker build -t <image-name>:<tag> .")}
												class="text-gray-400 hover:text-white opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
												title="Copy command"
											>
												<FiCopy size={16} />
											</button>
											<code>docker build -t &lt;image-name&gt;:&lt;tag&gt; .</code>
										</div>
									</div>

									{/* Step 2: Login */}
									<div>
										<p class="text-gray-300 text-sm mb-2">2. Login to Patr Registry</p>
										<div class="relative bg-secondary-light rounded px-4 py-3 font-mono text-sm text-gray-300 flex items-center gap-3 group">
											<button
												onClick={() => handleCopy(`docker login ${registryUrl} -u patr`)}
												class="text-gray-400 hover:text-white opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
												title="Copy command"
											>
												<FiCopy size={16} />
											</button>
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

									{/* Step 3: Tag */}
									<div>
										<p class="text-gray-300 text-sm mb-2">3. Tag your image</p>
										<div class="relative bg-secondary-light rounded px-4 py-3 font-mono text-sm text-gray-300 flex items-center gap-3 group">
											<button
												onClick={() => handleCopy(`docker tag <image-name>:<tag> ${repositoryPath()}:<tag>`)}
												class="text-gray-400 hover:text-white opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
												title="Copy command"
											>
												<FiCopy size={16} />
											</button>
											<code>docker tag &lt;image-name&gt;:&lt;tag&gt; {repositoryPath()}:&lt;tag&gt;</code>
										</div>
									</div>

									{/* Step 4: Push */}
									<div>
										<p class="text-gray-300 text-sm mb-2">4. Push the image</p>
										<div class="relative bg-secondary-light rounded px-4 py-3 font-mono text-sm text-gray-300 flex items-center gap-3 group">
											<button
												onClick={() => handleCopy(`docker push ${repositoryPath()}:<tag>`)}
												class="text-gray-400 hover:text-white opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
												title="Copy command"
											>
												<FiCopy size={16} />
											</button>
											<code>docker push {repositoryPath()}:&lt;tag&gt;</code>
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
											<button
												onClick={() => handleCopy(`docker login ${registryUrl} -u patr`)}
												class="text-gray-400 hover:text-white opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
												title="Copy command"
											>
												<FiCopy size={16} />
											</button>
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
											<button
												onClick={() => handleCopy(`docker tag <existing-image>:<tag> ${repositoryPath()}:<tag>`)}
												class="text-gray-400 hover:text-white opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
												title="Copy command"
											>
												<FiCopy size={16} />
											</button>
											<code>docker tag &lt;existing-image&gt;:&lt;tag&gt; {repositoryPath()}:&lt;tag&gt;</code>
										</div>
									</div>

									{/* Step 3: Push */}
									<div>
										<p class="text-gray-300 text-sm mb-2">3. Push the image</p>
										<div class="relative bg-secondary-light rounded px-4 py-3 font-mono text-sm text-gray-300 flex items-center gap-3 group">
											<button
												onClick={() => handleCopy(`docker push ${repositoryPath()}:<tag>`)}
												class="text-gray-400 hover:text-white opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
												title="Copy command"
											>
												<FiCopy size={16} />
											</button>
											<code>docker push {repositoryPath()}:&lt;tag&gt;</code>
										</div>
									</div>
								</div>
							</div>
						</div>
					</div>
				</div>
			</Show>
		</div>
	);
};

export default General;
