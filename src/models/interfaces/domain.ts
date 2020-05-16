export interface Domain {
	deploymentId: Buffer;
	domain: string;
	port: number;
	verified: 0 | 1;
	challenge: Buffer;
}
