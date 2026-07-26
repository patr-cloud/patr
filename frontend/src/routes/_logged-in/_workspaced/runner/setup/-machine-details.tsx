import { Show } from "solid-js";
import { GetRunnerLinkResponse } from "~/bindings";
import { formatRelativeTime } from "~/utils/func";
import { MapView } from "./-map-view";

const DetailCell = (props: { label: string; value: string }) => (
	<div class="flex flex-col gap-1 min-w-0">
		<span class="text-grey/70 text-xxs uppercase tracking-wider">{props.label}</span>
		<span class="font-log text-white text-sm truncate" title={props.value}>
			{props.value}
		</span>
	</div>
);

/**
 * The block describing the machine that's asking to be set up — version, OS,
 * IPs, geolocation + map. Rendered once at the top of the consent page and
 * shared across both the "new runner" and "reconnect" modes: it's what the
 * operator is authorizing regardless of which action they pick.
 */
export const MachineDetails = (props: { link: GetRunnerLinkResponse }) => {
	return (
		<div class="flex flex-col gap-6">
			<Show when={props.link.latitude && props.link.longitude}>
				<MapView lat={props.link.latitude!} lng={props.link.longitude!} />
			</Show>

			<section class="grid grid-cols-2 gap-x-8 gap-y-4">
				<DetailCell label="Version" value={props.link.version} />
				<DetailCell label="Started" value={formatRelativeTime(props.link.createdAt as unknown as string)} />
				<DetailCell label="OS" value={props.link.os} />
				<DetailCell label="Architecture" value={props.link.arch} />
				<DetailCell label="Hostname" value={props.link.hostname} />
				<DetailCell
					label="Location"
					value={[props.link.city, props.link.country].filter(Boolean).join(", ") || "Unknown"}
				/>
				<DetailCell label="Public IP" value={props.link.publicIp} />
				<DetailCell label="Private IP" value={props.link.privateIp} />
			</section>
		</div>
	);
};
