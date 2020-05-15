/*
 * Docker registry JWT granted claims format as per
 * https://docs.docker.com/registry/spec/auth/jwt/
* */
export interface RegistryClaim {
	type: 'repository';
	name: string;
	actions: ('pull' | 'push')[];
}

export type RegistryClaims = RegistryClaim[];
