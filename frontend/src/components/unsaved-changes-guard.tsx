import { Accessor, createEffect, mergeProps, onCleanup } from "solid-js";
import { useBlocker } from "@tanstack/solid-router";
import Modal, { ModalContainer } from "~/components/modal";
import Button from "~/components/button";
import { ButtonVariant, Color } from "~/utils/color";

interface UnsavedChangesGuardProps {
	/**
	 * Reactive condition that is true while there are unsaved changes. When true,
	 * in-app navigation is paused behind a confirmation modal and the browser's
	 * own prompt is armed for tab close / refresh.
	 */
	when: Accessor<boolean>;
	/** Heading shown in the confirmation modal. */
	title?: string;
	/** Body copy shown in the confirmation modal. */
	message?: string;
	/** Label for the dismiss button that keeps the user on the page. */
	stayLabel?: string;
	/** Label for the button that discards changes and navigates away. */
	leaveLabel?: string;
}

/**
 * Guards a page against navigating away with unsaved changes. Covers both
 * in-app navigation (via the router blocker + a confirmation modal) and the
 * browser's own tab close / refresh path (via a `beforeunload` listener, whose
 * prompt wording the browser controls, not us).
 *
 * Drop it anywhere with a reactive dirty condition: `<UnsavedChangesGuard when={isDirty} />`.
 */
const UnsavedChangesGuard = (rawProps: UnsavedChangesGuardProps) => {
	const props = mergeProps(
		{
			title: "Unsaved changes",
			message: "You have unsaved changes. If you leave now, they'll be lost.",
			stayLabel: "Stay",
			leaveLabel: "Leave",
		},
		rawProps
	);

	// In-app navigation (sidebar, tabs): pause it and let the modal below decide.
	// `withResolver` hands us `proceed`/`reset` to wire to the buttons.
	const blocker = useBlocker({ withResolver: true, shouldBlockFn: () => props.when() });

	// Tab close / refresh / external navigation: the browser can only show its own
	// generic prompt, and only if a beforeunload listener is registered — so add
	// one while dirty and tear it down once clean.
	createEffect(() => {
		if (typeof window === "undefined" || !props.when()) return;
		const handler = (e: BeforeUnloadEvent) => {
			e.preventDefault();
			e.returnValue = "";
		};
		window.addEventListener("beforeunload", handler);
		onCleanup(() => window.removeEventListener("beforeunload", handler));
	});

	return (
		<Modal
			isOpen={() => blocker().status === "blocked"}
			setIsOpen={(v) => {
				if (!v) blocker().reset?.();
			}}
			renderTrigger={() => null}
			renderModalContent={(close) => (
				<ModalContainer closeFn={() => close(false)} class="w-full">
					<h2 class="text-md text-primary font-semibold mb-4">{props.title}</h2>
					<p class="mb-6 text-sm text-white">{props.message}</p>
					<div class="flex w-full justify-end items-center gap-4">
						<Button variant={ButtonVariant.Plain} class="cursor-pointer" onClick={() => close(false)}>
							{props.stayLabel}
						</Button>
						<Button
							variant={ButtonVariant.Outlined}
							color={Color.Error}
							onClick={() => blocker().proceed?.()}
						>
							{props.leaveLabel}
						</Button>
					</div>
				</ModalContainer>
			)}
		/>
	);
};

export default UnsavedChangesGuard;
