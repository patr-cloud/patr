import { FiPlus, FiTrash2 } from "solid-icons/fi";
import { createSignal } from "solid-js";
import { EnvironmentVariableValue } from "~/bindings";
import { Button, ButtonVariant, Input, InputLabel, InputType } from "~/components";
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
}

const parseEnvValue = (value: EnvironmentVariableValue): string => {
	return typeof value === "string" ? value : JSON.stringify(value);
};

const EnvInput = (props: EnvInputProps) => {
	const [envName, setEnvName] = createSignal<string>("");
	const [envValue, setEnvValue] = createSignal<string>("");

	return (
		<div class="flex gap-8 items-start w-full">
			<InputLabel parentClass="flex-2 pt-3" label="Environment Variables" />

			<div class="flex flex-col flex-10 gap-4 w-full">
				{get(props.envList).map((env) => (
					<div class="flex items-center flex-10 gap-4">
						<Input
							disabled={true}
							class="flex-4"
							placeholder="Enter Env Name"
							type={InputType.Text}
							value={env.key}
							onInput={(e) => {
								setEnvName(e.currentTarget.value);
							}}
						/>
						<Input
							disabled={true}
							class="flex-7"
							placeholder="Enter Env Value"
							value={env.value ? parseEnvValue(env.value) : ""}
							type={InputType.Text}
							onInput={(e) => {
								setEnvValue(e.currentTarget.value);
							}}
						/>

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
					</div>
				))}
				<div class="flex items-center flex-10 gap-4">
					<Input
						class="flex-4"
						placeholder="Enter Env Name"
						type={InputType.Text}
						value={envName()}
						onInput={(e) => {
							setEnvName(e.currentTarget.value);
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
					/>

					<Button
						type="button"
						variant={ButtonVariant.Contained}
						class="flex-1 h-full flex items-center gap-2"
						onClick={(e) => {
							e.preventDefault();
							props.onAdd(envName(), envValue());
							setEnvName("");
							setEnvValue("");
						}}
					>
						<FiPlus size={16} />
					</Button>
				</div>
			</div>
		</div>
	);
};

export default EnvInput;
