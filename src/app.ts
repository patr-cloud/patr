import express from 'express';
import { join } from 'path';
import { EOL } from 'os';
import { existsSync, mkdirpSync, appendFile } from 'fs-extra-promise';
import { createStream } from 'rotating-file-stream';
import cookieParser from 'cookie-parser';
import logger from 'morgan';
import compression from 'compression';
import helmet from 'helmet';
import nunjucks from 'nunjucks';

import { messages, errors } from './config/errors';

import apiRouter from './routes/api.bytesonus.com';
import assetsRouter from './routes/assets.bytesonus.com';
import authRouter from './routes/auth.bytesonus.com';
import { basePath } from './config/config';

const app = express();

app.engine('html', nunjucks.render);
app.set('view engine', 'html');
nunjucks.configure(join(__dirname, 'views'), {
	noCache: true,
	autoescape: true,
	express: app,
});

// @if NODE_ENV != 'production'
app.locals.pretty = true;
app.use(logger('dev'));
app.set('json spaces', 4);
// @endif

/* @if NODE_ENV == 'production' **
const projectName = require('./package.json').name;
const logDirectory = join(process.env.HOME, '.logs', projectName);

app.use('/', compression());

app.locals.pretty = false;

// ensure log directory exists
if (existsSync(logDirectory) === false) {
	mkdirpSync(logDirectory);
}

app.use(logger(logger.compile(`:date, :method :req[Host]
:url :status :response-time ms - :res[content-length], :req[X-Forwarded-For]`), {
	stream: createStream(logFileNamer, {
		size: '200M',
		path: logDirectory,
		compress: true,
		immutable: true
	})
}));
app.set('json spaces', 0);
app.use(helmet());
/* @endif */

app.use(express.json());
app.use(express.urlencoded({ extended: false }));
app.use(cookieParser());

app.use((_req, res, next) => {
	res.header('Access-Control-Allow-Origin', '*');
	res.header('Access-Control-Allow-Methods', '*');
	res.header('Access-Control-Allow-Headers', 'Content-Type,Authorization');

	next();
});

app.use('/static', express.static(join(__dirname, 'static')));

app.use(basePath, (req, res, next) => {
	if (req.hostname === 'api.bytesonus.co') {
		apiRouter(req, res, next);
	} else if (req.hostname === 'assets.bytesonus.co') {
		assetsRouter(req, res, next);
	} else if (req.hostname === 'auth.bytesonus.co' || req.hostname === 'localhost') {
		authRouter(req, res, next);
	} else {
		next();
	}
});

// catch 404 and forward to error handler
app.use((_req, _res, next) => {
	next({
		status: 404,
		statusMessage: errors.notFound,
	});
});

// error handler
app.use((err: any, req: express.Request, res: express.Response, next: express.NextFunction) => {
	if (err.error && err.statusCode) {
		// JSON handler
		res.status(err.statusCode).json({
			success: false,
			error: err.error,
			message: messages[err.error],
		});
	} else {
		res.status(err.status || 500);
		res.json({
			success: false,
			error: err.statusMessage || errors.serverError,
			message: messages[err.statusMessage || errors.serverError],
		});
	}

	// @if NODE_ENV != 'production'
	if (err) {
		console.error(err);
	}
	// @endif
	/* @if NODE_ENV == 'production' **
	appendFile(join(logDirectory, 'error.log'), err.toString() + EOL + err.stack, err => {
		if (err) {
			console.error(err);
		}
	});
	/* @endif */

	next();
});

function logFileNamer(time: Date, index: number) {
	if (!time) {
		return 'access.log';
	}
	const year = time.getFullYear();
	const month = time.getMonth();
	const day = time.getDate();
	const hour = time.getHours();
	const minute = time.getMinutes();
	const seconds = time.getSeconds();

	if (index) {
		return `access - ${year}-${month + 1}-${day}, ${hour}:${minute}:${seconds}.${index}.log`;
	}
	return `access - ${year}-${month + 1}-${day}, ${hour}:${minute}:${seconds}.log`;
}

export default app;
