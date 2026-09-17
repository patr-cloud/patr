/**
 * Helpers for the `returnTo` search param, which carries where a user should
 * be sent back to after logging in.
 *
 * The param exists so an interrupted flow — currently only OAuth consent —
 * can resume after the login detour. It is user-controlled input that we then
 * navigate to, which makes it an open redirect unless it is constrained to
 * our own origin: `?returnTo=https://evil.example/session-expired` would
 * otherwise let an attacker hand out a genuine Patr login link that dumps the
 * user on a convincing fake once they have actually signed in.
 */

/**
 * Narrows a `returnTo` value to a path on this origin, or discards it.
 *
 * Parsed as a URL rather than pattern-matched on the string, so that anything
 * carrying its own host is rejected outright — including the protocol-relative
 * `//evil.example`, which reads like a path but resolves to another origin.
 */
export function sanitizeReturnTo(raw: string | undefined | null): string | undefined {
	if (!raw) {
		return undefined;
	}

	// Backslashes are normalised to forward slashes by browsers when
	// resolving a URL, so `/\evil.example` would escape the origin too. The
	// URL parser below does not do that normalisation for us.
	if (raw.includes("\\")) {
		return undefined;
	}

	let parsed: URL;
	try {
		// A base is required for a relative input. Anything absolute ignores
		// it, which is exactly how an off-origin value gets caught.
		parsed = new URL(raw, "https://patr.invalid");
	} catch {
		return undefined;
	}

	if (parsed.origin !== "https://patr.invalid") {
		return undefined;
	}

	return `${parsed.pathname}${parsed.search}${parsed.hash}`;
}

/** Builds a `returnTo` value for the page currently being viewed. */
export function buildReturnTo(location: { pathname: string; search?: string }): string {
	return `${location.pathname}${location.search ?? ""}`;
}
