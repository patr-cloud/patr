import { FiPlus, FiTrash2 } from "solid-icons/fi";
import { createSignal, Show, For } from "solid-js";
import { EnvironmentVariableValue } from "~/bindings";
import { Button, ButtonVariant, useToast } from "~/components";
import Input, { InputType } from "~/components/input";
import InputLabel from "~/components/input-label";
import { Color } from "~/utils/color";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface EnvInputProps {
	/** Additional Classes for the input.  */
	class?: MaybeAccessor<string>;
	/** On Add Value Handler */
	onAdd: (key: string, value: EnvironmentVariableValue) => void;
	/** On Remove Value Handler */
	onDelete: (key: string) => void;
	/** Env List */
	envList: MaybeAccessor<{ key: string; value?: EnvironmentVariableValue }[]>;
	/** Disabled state for the input */
	disabled?: MaybeAccessor<boolean>;
}

const parseEnvValue = (value: EnvironmentVariableValue): string => {
	return typeof value === "string" ? value : JSON.stringify(value);
};

const EnvInput = (props: EnvInputProps) => {
	const [envName, setEnvName] = createSignal<string>("");
	const [envValue, setEnvValue] = createSignal<string>("");
	const toast = useToast();

	return (
		<div class="flex gap-8 items-start w-full">
			<InputLabel parentClass="flex-2" label="Environment Variables" />

			<div class="flex flex-col flex-10 gap-4 w-full">
				<For each={get(props.envList)}>{(env) => (
					<div class="flex items-center flex-10 gap-4">
						<Input
							disabled={true}
							class="flex-4"
							placeholder="Enter Env Name"
							type={InputType.Text}
							value={env.key}
							onKeyDown={(e) => {
								if (e.key === "Enter") e.preventDefault();
							}}
						/>
						<Input
							disabled={get(props.disabled)}
							class="flex-7"
							placeholder="Enter Env Value"
							value={env.value ? parseEnvValue(env.value) : ""}
							type={InputType.Text}
							onInput={(e) => {
								props.onAdd(env.key, e.currentTarget.value);
							}}
							onKeyDown={(e) => {
								if (e.key === "Enter") e.preventDefault();
							}}
						/>

						<Show when={!get(props.disabled)}>
							<Button
								onClick={() => {
									props.onDelete(env.key);
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

				<Show
					when={!get(props.disabled)}
					fallback={
						<div class="flex items-center flex-10 gap-4">
							<Input
								class="flex-4"
								disabled={true}
								placeholder="Enter Env Name"
								type={InputType.Text}
								value={envName()}
							/>
							<Input
								class="flex-7"
								disabled={true}
								placeholder="Enter Env Value"
								type={InputType.Text}
								value={envValue()}
							/>
							<Button
								type="button"
								variant={ButtonVariant.Contained}
								disabled={true}
								class="flex-1 h-full flex items-center gap-2"
							>
								<FiPlus size={16} />
							</Button>
						</div>
					}
				>
					<div class="flex items-center flex-10 gap-4">
						<Input
							class="flex-4"
							placeholder="Enter Env Name"
							type={InputType.Text}
							value={envName()}
							onInput={(e) => {
								setEnvName(e.currentTarget.value);
							}}
							onKeyDown={(e) => {
								if (e.key === "Enter") {
									e.preventDefault();
									const envKey = envName();
									const envVal = envValue();
									if (!envKey || !envVal) {
										toast("Both Env Name and Value are required", "error");
										return;
									}
									props.onAdd(envKey, envVal);
									setEnvName("");
									setEnvValue("");
								}
							}}
						/>
						<Input
							class="flex-7"
							placeholder="Enter Env Value"
							value={envValue()}
							type={InputType.Text}
							onInput={(e) => {
								setEnvValue(e.currentTarget.value);
							}}
							onKeyDown={(e) => {
								if (e.key === "Enter") {
									e.preventDefault();
									const envKey = envName();
									const envVal = envValue();
									if (!envKey || !envVal) {
										toast("Both Env Name and Value are required", "error");
										return;
									}
									props.onAdd(envKey, envVal);
									setEnvName("");
									setEnvValue("");
								}
							}}
						/>

						<Button
							type="button"
							variant={ButtonVariant.Contained}
							class="flex-1 h-full flex items-center gap-2"
							onClick={(e) => {
								e.preventDefault();

								const envKey = envName();
								const envVal = envValue();

								if (!envKey || !envVal) {
									toast("Both Env Name and Value are required", "error");
									return;
								}

								props.onAdd(envName(), envValue());
								setEnvName("");
								setEnvValue("");
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

export default EnvInput;
