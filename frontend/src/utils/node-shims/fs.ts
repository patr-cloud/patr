/**
 * Browser stand-in for `node:fs`, aliased in for the **client** bundle only
 * (see `app.config.ts`). The SSR bundle keeps the real module.
 *
 * `@secretlint/secretlint-rule-preset-recommend` reaches for `fs.readFileSync`
 * in one place: its GCP rule re-reads the linted file from disk to check
 * whether it is a PKCS#12 key. That path is behind `await import("node:fs")`
 * inside a `try`/`catch` — the preset's own comment says "browser does not
 * have fs module" — so throwing here lands in that catch and the rule simply
 * declines to report.
 *
 * The import still has to *resolve* at build time, which is what this file is
 * for. See `./path.ts` for why both secretlint packages are version-pinned.
 */

/** Always throws — there is no file to read in a browser. */
export function readFileSync(): never {
	throw new Error("node:fs is not available in the browser");
}

export default { readFileSync };
