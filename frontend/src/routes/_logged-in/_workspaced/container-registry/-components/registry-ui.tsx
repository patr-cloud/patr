import { Match, Show, Switch } from "solid-js";
import { CopyableField } from "~/components";
import { ManifestKind, Platform } from "~/bindings";

/**
 * Render a platform as the familiar `os/arch` (or `os/arch/variant`) string,
 * e.g. `linux/amd64` or `linux/arm/v7`.
 */
export const formatPlatform = (platform: Platform): string =>
	`${platform.os}/${platform.architecture}${platform.variant ? `/${platform.variant}` : ""}`;

/**
 * Shorten a digest for display, keeping the algorithm prefix and the first
 * twelve hex characters, e.g. `sha256:a1b2c3d4e5f6…`.
 */
export const shortDigest = (digest: string): string => {
	const [algorithm, hex] = digest.split(":");
	if (!hex) return digest;
	return `${algorithm}:${hex.slice(0, 12)}…`;
};

/**
 * A one-glance summary of what a manifest runs on, for the platform column:
 * a single image shows its `os/arch`; a multi-arch index shows "Runs on N
 * platforms"; anything without a platform (an artifact) shows a dash.
 */
export const PlatformSummary = (props: { kind: ManifestKind; platforms: Platform[] }) => (
	<Switch>
		<Match when={props.kind === "index"}>
			<span class="text-gray-300 text-sm">
				<Show when={props.platforms.length > 0} fallback="Multi-arch">
					Runs on {props.platforms.length} platform{props.platforms.length === 1 ? "" : "s"}
				</Show>
			</span>
		</Match>
		<Match when={props.platforms.length > 0}>
			<span class="font-mono text-gray-300">{formatPlatform(props.platforms[0])}</span>
		</Match>
	</Switch>
);

/**
 * A plain-language label naming the manifest's shape for the cases the platform
 * column can't. Images and indexes are already covered by [`PlatformSummary`]
 * (`os/arch` and "Runs on N platforms"); only artifacts, which have no platform,
 * get a label here.
 */
export const KindBadge = (props: { kind: ManifestKind }) => (
	<Show when={props.kind === "artifact"}>
		<span class="text-gray-400 text-sm">Attached file</span>
	</Show>
);

/**
 * A copy-paste `docker pull` command for a fully-qualified image reference
 * (either `registry.patr.cloud/ws/repo:tag` or `…@sha256:…`).
 */
export const PullCommand = (props: { reference: string; label?: string }) => (
	<div>
		<Show when={props.label}>
			<p class="text-gray-300 text-sm mb-2">{props.label}</p>
		</Show>
		<CopyableField value={`docker pull ${props.reference}`} innerClass="font-mono" />
	</div>
);
