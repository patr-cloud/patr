import { FiChevronRight } from "solid-icons/fi";
import { Button, StatusBadge } from "~/components";

const RunnerCard = () => {
	return (
		<div class="bg-secondary-light flex flex-col items-start justify-start px-lg py-md br-sm text-white gap-xs rounded-xs">
			<div class="w-full flex items-center justify-between gap-md">
				<p class="text-md text-primary text-ellipsis overflow-hidden">Runner Name</p>

				<StatusBadge text="Live" />
			</div>

			<div class="flex-2 w-full gap-xs flex items-center justify-center rounded-xs">
				<div class="bg-secondary-medium br-sm px-lg py-sm flex flex-col items-start justify-center w-full">
					<small class="letter-sp-md text-xxs text-grey">LAST SEEN</small>
					<p class="text-primary w-[15ch] text-ellipsis overflow-hidden">Just Now</p>
				</div>
			</div>

			<Button class="text-medium text-primary letter-sp-md text-sm mt-xs ml-auto">
				MANAGE RUNNER
				<FiChevronRight />
			</Button>
		</div>
	);
};

export default RunnerCard;
