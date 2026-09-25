/**
 * Browser stand-in for `node:path`, aliased in for the **client** bundle only
 * (see `app.config.ts`). The SSR bundle keeps the real module.
 *
 * It exists for one dependency: `@secretlint/secretlint-rule-preset-recommend`
 * opens its ESM bundle with a top-level `import path from "node:path"`, which
 * a browser build cannot resolve. The preset uses exactly one function from it
 * — `basename`, to name the file in a finding's message — so that is all this
 * implements.
 *
 * ---
 *
 * **Both secretlint packages are pinned to an exact version on purpose.**
 *
 * `preset-recommend` ships ~880 KB of *bundled* third-party rule code with no
 * runtime dependencies of its own, so a version bump swaps out a large body of
 * vendored source in one go, and this shim only holds while its use of Node
 * built-ins stays as narrow as it is today. Before raising either version,
 * re-read the published bundle for new `node:*` imports and check that
 * `basename` is still the only thing it wants from this module.
 */

/** Everything after the last separator, with trailing separators ignored. */
export function basename(input: string, suffix?: string): string {
	const trimmed = input.replace(/[\\/]+$/, "");
	const name = trimmed.slice(trimmed.search(/[^\\/]*$/));
	return suffix && name !== suffix && name.endsWith(suffix) ? name.slice(0, -suffix.length) : name;
}

/** Everything after the last dot of the basename, including the dot. */
export function extname(input: string): string {
	const name = basename(input);
	const dot = name.lastIndexOf(".");
	return dot <= 0 ? "" : name.slice(dot);
}

/** The preset imports the module's default export, so it has to exist. */
export default { basename, extname };
