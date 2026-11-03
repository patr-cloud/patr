/** What each identity scope lets the client see. */
export const SCOPE_COPY: Record<string, { title: string; description: string }> = {
	openid: { title: "Sign you in", description: "Confirm who you are." },
	profile: { title: "Your name", description: "See your first and last name." },
	email: { title: "Your email address", description: "See the email address on your account." },
	offline_access: {
		title: "Stay signed in",
		description: "Keep access when you are not using the app, until you revoke it.",
	},
};
