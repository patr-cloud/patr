
export const port = '/* @echo PORT */';
export const basePath = '/* @echo BASE_PATH */';
export const passwordSaltRounds = 10;
export const jwtSecret = '/* @echo JWT_SECRET */';

export const s3 = {
	endpoint: '/* @echo S3_ENDPOINT */',
	region: '/* @echo S3_REGION */',
	bucket: '/* @echo S3_BUCKET */',
	key: '/* @echo S3_API_KEY */',
	secret: '/* @echo S3_API_SECRET */',
};

export const razorpay = {
	keyId: '/* @echo RAZORPAY_KEY_ID */',
	keySecret: '/* @echo RAZORPAY_KEY_SECRET */',
};

export const mysql = {
	host: '/* @echo MYSQL_HOST */',
	port: parseInt('/* @echo MYSQL_PORT */', 10),
	user: '/* @echo MYSQL_USER */',
	password: '/* @echo MYSQL_PASSWORD */',
	database: '/* @echo MYSQL_DATABASE */',
	connectionLimit: 10,
};

export const mongodb = {
	host: '/* @echo MONGODB_HOST */',
	port: parseInt('/* @echo MONGODB_PORT */', 10),
	database: '/* @echo MONGODB_DATABASE */',
};

export const email = {
	host: '/* @echo SMTP_HOST */',
	port: parseInt('/* @echo SMTP_PORT */', 10),
	secure: '/* @echo SMTP_SECURE */'.toLowerCase() === 'true',
	username: '/* @echo SMTP_USERNAME */',
	from: '/* @echo SMTP_FROM */',
	password: '/* @echo SMTP_PASSWORD */',
};

export const mailchimp = {
	username: '/* @echo MAILCHIMP_USERNAME */',
	dataCenter: '/* @echo MAILCHIMP_DATA_CENTER */',
	listId: '/* @echo MAILCHIMP_LIST_ID */',
	apiKey: '/* @echo MAILCHIMP_API_KEY */',
};

export const moduleSecretPrivateKey = '/* @echo MODULE_SECRET_PRIVATE_KEY */';
