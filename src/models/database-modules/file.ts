import { pool } from '../database';
import { FileACL } from '../interfaces/file-acl';
import { File } from '../interfaces/file';

export async function getAllowedGroupsForFile(fileId: string): Promise<FileACL[]> {
	const rows = await pool.query('SELECT * FROM file_acl_groups WHERE fileId = ?;', [fileId]);
	const groups: FileACL[] = [];
	for (let i = 0; i < rows.length; i++) {
		groups.push(rows[i]);
	}
	return groups;
}

export async function getFileInfo(fileId: string): Promise<File> {
	const rows = await pool.query('SELECT * FROM files WHERE fileId = ?;', [fileId]);
	return rows.length === 0 ? null : rows[0];
}

export async function createFile(fileId: string, contentType: string, fileName: string, size: number) {
	await pool.query(
		`
		INSERT INTO
			files (fileId, contentType, fileName, created, size)
		VALUES
			(?, ?, ?, ?, ?);
		`,
		[fileId, contentType, fileName, Date.now(), size]
	);
}

export async function deleteFile(fileId: string) {
	await pool.query('DELETE FROM files WHERE fileId = ?;', [fileId]);
}

export async function setFileHash(fileId: string, hash: string) {
	await pool.query('UPDATE files SET hash = ? WHERE fileId = ?;', [hash, fileId]);
}

export async function setFileData(fileId: string, contentType: string, fileName: string, size: number) {
	await pool.query(
		`
		UPDATE
			files
		SET
			contentType = ?,
			fileName = ?,
			size = ?
		WHERE
			fileId = ?
		`,
		[contentType, fileName, size, fileId]
	);
}
