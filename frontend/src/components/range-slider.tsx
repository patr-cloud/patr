import { createMemo, For, mergeProps, Show } from "solid-js";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface RangeSliderProps {
	/** Track minimum value */
	min: number;
	/** Track maximum value */
	max: number;
	/** Current low value */
	valueLow: MaybeAccessor<number>;
	/** Current high value */
	valueHigh: MaybeAccessor<number>;
	/** Called when low value changes */
	onChangeLow: (val: number) => void;
	/** Called when high value changes */
	onChangeHigh: (val: number) => void;
	/** Whether the slider is disabled */
	disabled?: boolean;
	/** Step increment, defaults to 1 */
	step?: number;
	/** Additional classes for the container */
	class?: string;
}

const RangeSlider = (rawProps: RangeSliderProps) => {
	const props = mergeProps(
		{
			step: 1,
			disabled: false,
			class: "",
		},
		rawProps
	);

	const low = () => get(props.valueLow);
	const high = () => get(props.valueHigh);
	const range = () => props.max - props.min;

	const lowPercent = createMemo(() => ((low() - props.min) / range()) * 100);
	const highPercent = createMemo(() => ((high() - props.min) / range()) * 100);

	const ticks = createMemo(() => {
		const result: number[] = [];
		for (let i = props.min; i <= props.max; i += props.step) {
			result.push(i);
		}
		return result;
	});

	const handleLowInput = (e: Event) => {
		const target = e.target as HTMLInputElement;
		const val = Number(target.value);
		if (val <= high()) {
			props.onChangeLow(val);
		} else {
			props.onChangeLow(high());
			target.value = String(high());
		}
	};

	const handleHighInput = (e: Event) => {
		const target = e.target as HTMLInputElement;
		const val = Number(target.value);
		if (val >= low()) {
			props.onChangeHigh(val);
		} else {
			props.onChangeHigh(low());
			target.value = String(low());
		}
	};

	return (
		<div class={`flex flex-col gap-2 ${props.class}`}>
			{/* Value badges */}
			<div class="flex items-center justify-center gap-4">
				<div class="flex items-center gap-1.5">
					<span class="text-[11px] text-white/40 uppercase tracking-wide">Min</span>
					<span class="font-log text-sm font-medium text-primary bg-primary/8 border border-primary/15 px-2.5 py-0.5 rounded-xs">
						{low()}
					</span>
				</div>
				<div class="flex items-center gap-1.5">
					<span class="text-[11px] text-white/40 uppercase tracking-wide">Max</span>
					<span class="font-log text-sm font-medium text-primary bg-primary/8 border border-primary/15 px-2.5 py-0.5 rounded-xs">
						{high()}
					</span>
				</div>
			</div>

			{/* Track with overlaid range inputs */}
			<div class="relative flex items-center h-7">
				{/* Background track */}
				<div class="w-full h-0.75 bg-secondary-medium rounded-full" />

				{/* Active fill between thumbs */}
				<div
					class="absolute h-0.75 bg-primary/50 rounded-full"
					style={{
						left: `${lowPercent()}%`,
						right: `${100 - highPercent()}%`,
					}}
				/>

				{/* When values overlap: put low on top everywhere except at the min
					end of the range (where only dragging right makes sense) */}
				<input
					type="range"
					min={props.min}
					max={props.max}
					step={props.step}
					value={low()}
					disabled={props.disabled}
					onInput={handleLowInput}
					class="range-input absolute left-0 w-full pointer-events-none"
					style={{ "z-index": low() === high() && low() > props.min ? 4 : 3 }}
				/>

				<input
					type="range"
					min={props.min}
					max={props.max}
					step={props.step}
					value={high()}
					disabled={props.disabled}
					onInput={handleHighInput}
					class="range-input absolute left-0 w-full pointer-events-none"
					style={{ "z-index": low() === high() && low() > props.min ? 3 : 4 }}
				/>
			</div>

			{/* Tick marks */}
			<Show when={ticks().length <= 20}>
				<div class="flex justify-between px-0.5">
					<For each={ticks()}>{(tick) => <span class="text-xxs text-white/20 font-log">{tick}</span>}</For>
				</div>
			</Show>
		</div>
	);
};

export default RangeSlider;
