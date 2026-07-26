import { For } from "solid-js";

type Mode = "new" | "reconnect";

const CHOICES: { mode: Mode; glyph: string; title: string; body: string }[] = [
	{
		mode: "new",
		glyph: "✦",
		title: "New runner",
		body: "Register this machine as a brand-new runner in the workspace.",
	},
	{
		mode: "reconnect",
		glyph: "⟳",
		title: "Reconnect",
		body: "Replace the credentials of an existing runner with this machine.",
	},
];

/**
 * The landing step of the consent page. The operator explicitly picks whether
 * this machine becomes a new runner or takes over an existing one — reconnect
 * rotates credentials, so it should never be the default-rendered form.
 */
export const ModeChoice = (props: { onPick: (mode: Mode) => void }) => {
	return (
		<div class="flex flex-col gap-4">
			<h2 class="text-white text-lg font-semibold">What would you like to do?</h2>
			<div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
				<For each={CHOICES}>
					{(choice) => (
						<button
							type="button"
							onClick={() => props.onPick(choice.mode)}
							class="flex flex-col gap-3 text-left p-5 rounded-xs border border-border-color hover:border-primary hover:bg-primary/5 transition-colors cursor-pointer"
						>
							<span class="text-primary text-2xl leading-none">{choice.glyph}</span>
							<span class="text-white text-base font-medium">{choice.title}</span>
							<span class="text-grey text-sm leading-relaxed">{choice.body}</span>
						</button>
					)}
				</For>
			</div>
		</div>
	);
};
