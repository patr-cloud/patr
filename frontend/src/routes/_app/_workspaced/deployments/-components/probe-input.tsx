import { createSignal, Signal } from "solid-js";
import { DeploymentProbe } from "~/bindings";
import { InputDropdown } from "~/components";
import Input, { InputType } from "~/components/input";
import InputLabel from "~/components/input-label";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface ProbeInputProps {
	/** Additional Classes for the input.  */
	class?: MaybeAccessor<string>;
	/** Probe */
	probe: Signal<DeploymentProbe | undefined>;
	/** Existing Ports */
	ports: MaybeAccessor<number[]>;
}

const ProbeInput = (props: ProbeInputProps) => {
	const [path, setPath] = createSignal<string>("");

	const [probe, setProbe] = props.probe;

	const onSelectPort = (port: string) => {
		const probeVal = probe();

		if (!probeVal?.path || !path) return;

		setProbe({
			path: probeVal.path || path(),
			port: parseInt(port),
		});
	};

	return (
		<div class={`${get(props.class)} flex gap-8 items-start w-full`}>
			<InputLabel parentClass="flex-2 pt-3" label="Startup Probe" />

			<div class="flex flex-10 gap-4 w-full">
				<Input
					class="flex-1"
					value={probe()?.path ?? ""}
					placeholder="Enter Probe Path"
					id="probe-path"
					name="probe-path"
					type={InputType.Text}
					onInput={(e) => {
						const probeVal = probe();
						if (probeVal) {
							setProbe({
								...probeVal,
								path: e.currentTarget.value,
							});
						} else {
							setPath(e.currentTarget.value);
						}
					}}
				/>

				<InputDropdown
					class="flex-1"
					disabled={get(props.ports).length === 0 || (!probe()?.path && !path())}
					value={probe()?.port?.toString()}
					placeholder="Enter Probe Port"
					id="probe-port"
					name="probe-port"
					options={get(props.ports).map((port) => ({
						value: port.toString(),
						label: port.toString(),
					}))}
					onSelect={onSelectPort}
				/>
			</div>
		</div>
	);
};

export default ProbeInput;
