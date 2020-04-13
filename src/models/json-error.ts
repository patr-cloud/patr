export class JsonError extends Error {
	public error: string;
	public statusCode: number;

	constructor(status: number, error: string) {
		super(error);
		this.error = error;
		this.statusCode = status;
	}
}
