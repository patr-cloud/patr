
export interface File {
	fileId: string;
	contentType: string;
	fileName: string;
	created: number;
	hash: string; // smoke it up
	size: number;
}
