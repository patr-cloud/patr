import { createFileRoute } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { For } from "solid-js";
import { CopyableField, CopyableFieldVariant, PageContainer, PageContainerBody, PageContainerHead } from "~/components";

const SETUP_COMMAND = "patr runner setup";

const STEPS = [
	{
		idx: "1",
		title: "Run on the host",
		body: "Install the Patr CLI on the machine that will host the runner, then invoke setup from a terminal there.",
	},
	{
		idx: "2",
		title: "Approve in browser",
		body: "The CLI opens this dashboard with a one-time code. Confirm the machine details and assign a name.",
	},
	{
		idx: "3",
		title: "Runner online",
		body: "The CLI writes a per-runner credential and connects. The new runner appears in your list.",
	},
];

const CreateRunnerPage = () => {
	return (
		<>
			<Title>New Runner | Patr</Title>
			<PageContainer>
				<PageContainerHead
					subText="Runners execute deployments on your machines or clusters"
					breadcrumbs={[{ label: "Runners", url: "/runners" }, { label: "Add" }]}
				/>
				<PageContainerBody class="w-full">
					<div class="flex flex-col gap-8 w-full">
						<section>
							<h2 class="text-white text-lg font-semibold mb-2">Setup runs in your terminal</h2>
							<p class="text-grey text-sm leading-relaxed" style={{ "max-width": "65ch" }}>
								We hand setup off to the Patr CLI on the host machine. The runner gets its own
								credential — never your account token, never anything pasted around.
							</p>
						</section>

						<section>
							<h3 class="text-white text-base font-medium mb-4">Run this on the host machine</h3>
							<CopyableField value={SETUP_COMMAND} variant={CopyableFieldVariant.Input} />
						</section>

						<section>
							<h3 class="text-white text-base font-medium mb-4">What happens next</h3>
							<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
								<For each={STEPS}>
									{(step) => (
										<div class="border border-border-color rounded-xs p-5 flex flex-col gap-3">
											<div class="flex items-center gap-3">
												<span class="w-6 h-6 rounded-full border border-primary text-primary text-xs font-medium flex items-center justify-center">
													{step.idx}
												</span>
												<span class="text-white text-sm font-medium">{step.title}</span>
											</div>
											<p class="text-grey text-sm leading-relaxed">{step.body}</p>
										</div>
									)}
								</For>
							</div>
						</section>
					</div>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/runners/new")({
	component: CreateRunnerPage,
});
