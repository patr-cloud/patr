
export interface PasswordResetRequest {
	userId: string;
	token: string;
	tokenExpiry: number;
}
