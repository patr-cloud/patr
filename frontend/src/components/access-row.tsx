import { Show } from "solid-js";
import Link from "~/components/link";
import { Scope } from "~/utils/scope";

interface AccessRowProps {
	/** What is granted, or who holds it: a role, a member or a service account. */
	label: string;
	/** Where the grant applies. */
	scope: Scope;
	/** Makes the label a link, e.g. to the holder's own page. */
	href?: string;
}

/** One read-only grant: what is granted or who holds it, and where it applies. */
const AccessRow = (props: AccessRowProps) => {
	const scopeLabel = () => {
		if (props.scope.scopeType !== "resources") return "Entire workspace";
		const count = props.scope.resources.length;
		return `${count} resource${count === 1 ? "" : "s"}`;
	};

	return (
		<li class="flex items-center justify-between gap-3 px-3 py-2 border border-border-color rounded-xs">
			<Show when={props.href} fallback={<span class="text-white text-sm truncate">{props.label}</span>}>
				{(href) => (
					<Link href={href()} class="text-sm truncate justify-start min-w-0">
						{props.label}
					</Link>
				)}
			</Show>
			<span class="text-grey text-xs shrink-0">{scopeLabel()}</span>
		</li>
	);
};

export default AccessRow;
