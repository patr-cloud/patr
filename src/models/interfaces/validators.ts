
export function isEmailValid(email: string): boolean {
	const regex = /^(([^<>()[\]\\.,;:\s@"]+(\.[^<>()[\]\\.,;:\s@"]+)*)|(".+"))@((\[[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\])|(([a-zA-Z\-0-9]+\.)+[a-zA-Z]{2,}))$/;
	return regex.test(email.toLowerCase());
}

export function isUsernameValid(username: string): boolean {
	const regex = /^([a-z\d]+-)*[a-z\d]+$/i;
	return username.length <= 39 && regex.test(username);
}

export function isPasswordValid(password: string): boolean {
	const regex = /(?=.*\d)(?=.*[a-z])/;
	return password.length >= 8 && regex.test(password);
}
