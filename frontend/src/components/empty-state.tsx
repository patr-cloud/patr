import { For, JSX, Show } from "solid-js";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface EmptyStateProps {
	/** The heading text, e.g. "No Deployments Added" */
	title: MaybeAccessor<string>;
	/** Optional description text below the title */
	description?: MaybeAccessor<string>;
	/** Optional action element (e.g. a CTA button) */
	action?: JSX.Element;
}

const stars = Array.from({ length: 80 }, () => ({
	top: `${Math.random() * 50}%`,
	left: `${Math.random() * 100}%`,
	size: Math.random() * 2 + 0.5,
	delay: `${Math.random() * 3}s`,
	duration: `${Math.random() * 2 + 1.5}s`,
}));

const EmptyState = (props: EmptyStateProps) => {
	return (
		<div class="absolute inset-0 flex flex-col items-center rounded-b-xs overflow-hidden isolate">
			{/* Scattered stars — behind everything */}
			<div class="absolute inset-0 -z-10 pointer-events-none" aria-hidden="true">
				<For each={stars}>
					{(star) => (
						<div
							class="absolute bg-white rounded-full animate-pulse"
							style={{
								top: star.top,
								left: star.left,
								width: `${star.size}px`,
								height: `${star.size}px`,
								"animation-delay": star.delay,
								"animation-duration": star.duration,
							}}
						/>
					)}
				</For>
			</div>

			{/* Text content — sits above the image */}
			<div class="relative flex flex-col items-center gap-3 md:gap-4 pt-8 md:pt-16 pb-4 md:pb-8 px-4 z-10 text-center max-w-full">
				<h2 class="text-lg md:text-xl font-medium text-white">{get(props.title)}</h2>
				<Show when={props.description}>
					<p class="text-xs md:text-sm text-grey">{get(props.description!)}</p>
				</Show>
				<Show when={props.action}>
					<div class="mt-2 bg-secondary-dark">{props.action}</div>
				</Show>
			</div>

			{/* Jeep illustration */}
			<img
				src="/images/jeep.png"
				alt="No records found image"
				class="relative w-full mt-auto object-cover object-top select-none pointer-events-none z-10"
			/>
		</div>
	);
};

export default EmptyState;
