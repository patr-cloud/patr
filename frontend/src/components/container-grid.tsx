import { JSX, mergeProps, ParentProps } from "solid-js";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface ContainerGridProps<ItemI> {
	/** Additional Classes for the container grid. */
	class?: string;
	/** Render Card */
	renderCard: (item: ItemI) => JSX.Element;
	/** Items to render */
	items: MaybeAccessor<ItemI[]>;
}

const ContainerGrid = <ItemI,>(rawProps: ContainerGridProps<ItemI>) => {
	const props = mergeProps(
		{
			class: "",
		},
		rawProps
	);
	return (
		<section class="w-full overflow-y-auto">
			<div class={`grid grid-cols-3 gap-xl justify-start content-start ${get(props.class)}`}>
				{get(props.items).map((item) => props.renderCard(item))}
			</div>
		</section>
	);
};

export default ContainerGrid;
