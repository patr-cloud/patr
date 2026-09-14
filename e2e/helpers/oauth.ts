import { createHash, randomBytes } from 'node:crypto';
import { API_DIRECT_URL } from '@/helpers/urls';

/**
 * The OAuth client seeded for these specs by `e2e/Justfile`. Its redirect URI
 * points at a port nothing listens on — the specs intercept it, because what
 * is being asserted is the URL the browser gets sent to, not what answers
 * there.
 */
export const E2E_CLIENT_ID = 'e2e';
export const E2E_REDIRECT_URI = 'http://localhost:19999/cb';

function base64Url(input: Buffer): string {
	return input.toString('base64').replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

/** A PKCE verifier/challenge pair, S256 — the only method OAuth 2.1 allows. */
export function createPkcePair(): { verifier: string; challenge: string } {
	const verifier = base64Url(randomBytes(64));
	return { verifier, challenge: base64Url(createHash('sha256').update(verifier).digest()) };
}

/**
 * Drives `/authorize` far enough to get a parked request, and returns the id
 * the consent page is addressed by.
 *
 * Deliberately does not follow the redirect: the `Location` *is* the result,
 * and following it here would consume the very navigation a spec wants to
 * make in a browser.
 */
export async function startAuthorization(
	options: { state?: string; scope?: string } = {},
): Promise<{ requestId: string; state: string; verifier: string }> {
	const { challenge, verifier } = createPkcePair();
	const state = options.state ?? base64Url(randomBytes(8));

	const query = new URLSearchParams({
		response_type: 'code',
		client_id: E2E_CLIENT_ID,
		redirect_uri: E2E_REDIRECT_URI,
		scope: options.scope ?? 'openid profile email',
		state,
		code_challenge: challenge,
		code_challenge_method: 'S256',
	});

	const response = await fetch(`${API_DIRECT_URL}/auth/oauth/authorize?${query}`, {
		redirect: 'manual',
	});

	const location = response.headers.get('location');
	if (!location) {
		throw new Error(
			`/authorize did not redirect (status ${response.status}): ${await response.text()}`,
		);
	}

	const requestId = new URL(location, 'http://localhost').searchParams.get('requestId');
	if (!requestId) {
		throw new Error(`/authorize redirected without a requestId: ${location}`);
	}

	return { requestId, state, verifier };
}
