import { FiExternalLink, FiPlus, FiTrash2 } from "solid-icons/fi";
import { createSignal, Show, For } from "solid-js";
import { ExposedPortType } from "~/bindings";
import { Button, ButtonVariant, Input, InputDropdown, InputLabel, useToast } from "~/components";
import { Color } from "~/utils/color";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface PortInputProps {
	/** Additional Classes for the input.  */
	class?: MaybeAccessor<string>;
	/** On Add Port Handler */
	onAdd: (key: string, value: ExposedPortType) => void;
	/** On Remove Port Type Handler */
	onDelete: (key: string) => void;
	/** Port List */
	portList: MaybeAccessor<{ [key: string]: ExposedPortType | undefined }>;
	/** Deployment ID for HTTP Port URL generation. This should show up only when the deployment is being edited */
	deploymentId?: string;
	/** Disabled state for the input */
	disabled?: MaybeAccessor<boolean>;
}

const PortInput = (props: PortInputProps) => {
	const [portNumber, setPortNumber] = createSignal<string>("");
	const [portType, setPortType] = createSignal<ExposedPortType | undefined>(undefined);

	const toast = useToast();

	return (
		<div class={`${get(props.class)} flex gap-8 items-start w-full`}>
			<InputLabel parentClass="flex-2 pt-3" label="Exposed Ports" />

			<div class="flex flex-col flex-10 gap-4 w-full">
				<For each={Object.entries(get(props.portList))}>{([port, portType]) => (
					<div class="flex items-center flex-10 gap-4 w-full">
						<Input class="flex-6" disabled={true} value={port} />
						<Input
							class={portType === "http" && props.deploymentId ? "flex-3" : "flex-5"}
							disabled={true}
							value={portType}
						/>
						{portType === "http" && props.deploymentId && (
							<a
								class="flex-2 flex items-center justify-start gap-2 rounded-xs bg-secondary-light border border-secondary-medium py-xs px-lg text-primary"
								href={`https://${port}-${props.deploymentId}.onpatr.cloud`}
								target="_blank"
							>
								<FiExternalLink size={16} />
								Visit URL
							</a>
						)}

						<Show when={!get(props.disabled)}>
							<Button
								onClick={() => {
									props.onDelete(port);
								}}
								variant={ButtonVariant.Outlined}
								class="flex-1 h-full flex items-center gap-2"
								color={Color.Error}
							>
								<FiTrash2 size={16} />
							</Button>
						</Show>
					</div>
				)}</For>

				<Show when={!get(props.disabled)}>
					<div class="flex items-center flex-10 gap-4 w-full">
						<Input
							onInput={(e) => setPortNumber(e.currentTarget.value)}
							class="flex-6"
							placeholder="Enter Port Number"
						/>
						<InputDropdown
							placeholder="Select Port Type"
							value={portType()}
							class="flex-5"
							onSelect={(value) => {
								setPortType(value as ExposedPortType);
								const envVal = get(portType);
								const portVal = portNumber();

								if (envVal && portVal) {
									props.onAdd(portVal, envVal);
									setPortNumber("");
									setPortType(undefined);
								}
							}}
							options={[
								{
									value: "udp",
									label: "UDP",
								},
								{
									value: "tcp",
									label: "TCP",
								},
								{
									value: "http",
									label: "HTTP",
								},
							]}
						/>

						<Button
							type="button"
							variant={ButtonVariant.Contained}
							class="flex-1 h-full flex items-center gap-2"
							onClick={(e) => {
								e.preventDefault();
								const envVal = get(portType);
								const portVal = portNumber();

								if (!envVal || !portVal) {
									toast("Both Port Number and Port Type are required", "error");
									return;
								}
								props.onAdd(portNumber(), envVal);
								setPortNumber("");
								setPortType(undefined);
							}}
						>
							<FiPlus size={16} />
						</Button>
					</div>
				</Show>
			</div>
		</div>
	);
};

export default PortInput;
