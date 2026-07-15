// Lazy loader for the zxcvbn password-strength estimator. The core library is
// small, but its dictionary (@zxcvbn-ts/language-common) is large (~200KB gzip),
// so both are pulled in via dynamic import() and only when first needed (when
// the user starts typing a password). This keeps them out of the initial
// bundle — nothing imports this module statically except the on-focus caller.

type ScoreFn = (password: string) => number;

let scoreFn: ScoreFn | undefined;
let loading: Promise<ScoreFn> | undefined;

/** The loaded scorer, or `undefined` if it hasn't finished loading yet. */
export function getEstimator(): ScoreFn | undefined {
	return scoreFn;
}

/**
 * Loads (once) and returns the zxcvbn scorer. Safe to call repeatedly — the
 * dynamic import and factory construction happen a single time; subsequent
 * calls resolve to the cached function. Resolves to a function returning a 0–4
 * strength score for a password. No-op safety: only invoke from client code
 * (it dynamically imports browser-oriented packages).
 */
export function loadEstimator(): Promise<ScoreFn> {
	if (scoreFn) return Promise.resolve(scoreFn);
	if (loading) return loading;

	loading = (async () => {
		const [core, common] = await Promise.all([import("@zxcvbn-ts/core"), import("@zxcvbn-ts/language-common")]);
		const zxcvbn = new core.ZxcvbnFactory({
			dictionary: { ...common.dictionary },
			graphs: common.adjacencyGraphs,
		});
		scoreFn = (password: string) => zxcvbn.check(password).score;
		return scoreFn;
	})();

	return loading;
}
