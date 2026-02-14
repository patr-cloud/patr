import { Show } from "solid-js";
import { FiCopy } from "solid-icons/fi";
import { GetContainerRepositoryInfoResponse } from "~/bindings";
import { Input, InputLabel, InputType, useToast } from "~/components";
import { parseDate } from "~/utils/func";

interface GeneralInfoProps {
	repositoryInfo: GetContainerRepositoryInfoResponse | undefined;
}

const General = (props: GeneralInfoProps) => {
	return (
		<div class="w-full">
			<Show when={props.repositoryInfo} fallback={<div class="text-gray-400 p-6">Loading...</div>}>
				<div class="p-6 space-y-6">
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
									value={props.repositoryInfo?.repository?.size as string | undefined}
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
								<Input
									value={
										parseDate(props.repositoryInfo?.repository?.lastUpdated || "")?.toLocaleString("en-US", {
											year: "numeric",
											month: "short",
											day: "numeric",
											hour: "2-digit",
											minute: "2-digit",
											second: "2-digit",
										}) || "N/A"
									}
									disabled={true}
									class="flex-10"
									name="repository-last-updated"
									placeholder="Repository last updated"
									type={InputType.Text}
								/>
							</div>

							<div class="flex items-center gap-4">
								<InputLabel parentClass="flex-2" for="repository-created" label="Created" />
								<Input
									value={
										parseDate(props.repositoryInfo?.repository?.created || "")?.toLocaleString("en-US", {
											year: "numeric",
											month: "short",
											day: "numeric",
											hour: "2-digit",
											minute: "2-digit",
											second: "2-digit",
										}) || "N/A"
									}
									disabled={true}
									class="flex-10"
									name="repository-created"
									placeholder="Repository created"
									type={InputType.Text}
								/>
							</div>
						</div>
					</div>
				</div>
			</Show>
		</div>
	);
};

export default General;
