import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface EmptyStateProps {
	/** The heading text, e.g. "No Deployments Added" */
	title: MaybeAccessor<string>;
}

const EmptyState = (props: EmptyStateProps) => {
	return (
		<div class={`relative flex flex-col items-center flex-1 -mx-md -mb-md rounded-b-xs overflow-hidden`}>
			{/* Text content — sits above the image */}
			<div class="flex flex-col items-center gap-4 pt-16 pb-8 z-10">
				<h2 class="text-xl font-medium text-white">{get(props.title)}</h2>
			</div>

			{/* Jeep illustration */}
			<img
				src="/images/jeep.png"
				alt="No records found image"
				class="w-full mt-auto object-cover object-top select-none pointer-events-none"
			/>
		</div>
	);
};

export default EmptyState;
