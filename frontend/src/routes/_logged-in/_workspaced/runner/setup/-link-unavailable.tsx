import { useNavigate } from "@tanstack/solid-router";
import { Show } from "solid-js";
import { Alert, Button, ButtonVariant } from "~/components";

export const LinkUnavailable = (props: { showWorkspaceHint: boolean }) => {
	const navigate = useNavigate();
	return (
		<div class="flex flex-col items-center justify-center gap-4 py-16 w-full">
			<h2 class="text-white text-lg font-semibold">This link can't be used</h2>
			<p class="text-grey text-sm text-center leading-relaxed" style={{ "max-width": "32rem" }}>
				The link may have expired (5-minute window), already been claimed, or been created in a different
				workspace. Run <code class="font-log text-white">patr runner setup</code> again on the host machine to
				start a fresh handshake.
			</p>
			<Show when={props.showWorkspaceHint}>
				<Alert
					align="center"
					type="warning"
					message="Double-check you're in the right workspace. A code created for one workspace won't work with another."
				/>
			</Show>
			<Button variant={ButtonVariant.Outlined} onClick={() => navigate({ to: "/runner/setup", search: {} })}>
				Enter a different code
			</Button>
		</div>
	);
};
