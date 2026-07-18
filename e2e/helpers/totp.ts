import { TOTP, Secret } from 'otpauth';

// Matches the API's totp-rs config in api/src/routes/.../login.rs:131-174:
// SHA-1, 30s period, 6 digits, current step.
export function computeTotp(base32Secret: string): string {
	const totp = new TOTP({
		algorithm: 'SHA1',
		digits: 6,
		period: 30,
		secret: Secret.fromBase32(base32Secret),
	});
	return totp.generate();
}
