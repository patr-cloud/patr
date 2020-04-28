
export const port = '/* @echo PORT */';
export const basePath = '/* @echo BASE_PATH */';
export const passwordSaltRounds = 10;
export const jwtSecret = '/* @echo JWT_SECRET */';

export const mysql = {
	host: '/* @echo MYSQL_HOST */',
	port: parseInt('/* @echo MYSQL_PORT */', 10),
	user: '/* @echo MYSQL_USER */',
	password: '/* @echo MYSQL_PASSWORD */',
	database: '/* @echo MYSQL_DATABASE */',
	connectionLimit: 10,
};
