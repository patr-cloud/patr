export interface Client {
	clientId: string;
	clientSecret: string;
	redirectUris: string[];
	responseTypes: string[];
	grantTypes: string[];
}
