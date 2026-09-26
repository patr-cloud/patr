/**
 * Runs secretlint's recommended rule preset over a single `KEY=value` pair.
 *
 * The preset carries ~29 vendor rules (AWS, GitHub, Stripe, OpenAI, private
 * keys, connection strings, …) that would otherwise be ours to write and keep
 * current. It deliberately has no generic heuristics, so callers pair it with
 * their own — see `looksLikeSecret` in the deployments env list.
 *
 * The secretlint packages are pinned to an exact version; the reason, and what
 * to check before bumping them, is in `./node-shims/path.ts`.
 */

/** Loaded once, on first use, so the preset never enters the main bundle. */
let linter: Promise<(key: string, value: string) => Promise<string[]>> | null = null;

const loadLinter = async () => {
	const [{ lintSource }, preset, { secretLintProfiler }] = await Promise.all([
		import("@secretlint/core"),
		import("@secretlint/secretlint-rule-preset-recommend"),
		import("@secretlint/profiler"),
	]);

	// Core times every rule into the page's performance timeline by default and
	// never clears it. Nothing reads those timings, and the env list re-lints on
	// every pause in typing, so they would pile up for the life of the page. This
	// is the same instance core imports, so switching it off here covers core.
	secretLintProfiler.setEnabled(false);

	const config = {
		rules: [
			{
				id: "@secretlint/secretlint-rule-preset-recommend",
				rule: preset.creator,
			},
		],
	};

	return async (key: string, value: string): Promise<string[]> => {
		// Linting `KEY=value` against a `.env` path rather than the bare value:
		// several rules read the variable name for context.
		const result = await lintSource({
			source: { content: `${key}=${value}`, filePath: ".env", contentType: "text" },
			// Rule messages interpolate what they matched — `found PostgreSQL
			// connection string: postgresql://user:password@host`. Masking turns
			// that into asterisks before it can reach a screen or a log.
			options: { config, maskSecrets: true },
		});

		return result.messages.map((message) => describe(message.message));
	};
};

/** A run of asterisks, which is what masking leaves behind. */
const MASKED = /\*{3,}/g;

/**
 * Reduces a rule's message to the part that names what was found, dropping the
 * masked value itself: `found Stripe secret key: ****` becomes `found Stripe
 * secret key`. The asterisk run still encodes the secret's length, and reads
 * like noise either way.
 */
const describe = (message: string): string =>
	message
		.replace(MASKED, "")
		.replace(/\s+/g, " ")
		.replace(/[\s:,-]+$/, "")
		.trim();

/**
 * The findings for one environment variable, or an empty array when secretlint
 * has nothing to say. Never throws: a failure to load or lint just means no
 * findings, since this only drives an optional hint.
 */
export async function lintEnvVar(key: string, value: string): Promise<string[]> {
	try {
		const lint = await (linter ??= loadLinter());
		return await lint(key, value);
	} catch (error) {
		console.error("secretlint failed", error);
		return [];
	}
}
