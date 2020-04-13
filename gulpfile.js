let gulp = require('gulp');
let htmlmin = require('gulp-htmlmin');
let scss = require('gulp-sass');
let minifyCSS = require('gulp-csso');
let uglify = require('gulp-uglify-es').default;
let minify = require('gulp-minify');
let preprocess = require('gulp-preprocess');
let prettyData = require('gulp-pretty-data');
let clean = require('gulp-clean');
let newer = require('gulp-newer');
let ts = require('gulp-typescript');

let tsProject = ts.createProject("tsconfig.json");

gulp.task('html', () => {
	return gulp.src([
			'src/**/*.html',
			'!src/**/node_modules/',
			'!src/**/node_modules/**/*'
		])
		.pipe(newer('bin'))
		.pipe(htmlmin({
			collapseWhitespace: true,
			minifyCSS: true,
			minifyJS: true
		}))
		.pipe(gulp.dest('bin'));
});

gulp.task('scss', () => {
	return gulp.src([
			'src/**/*.{scss,sass}',
			'!src/**/node_modules/',
			'!src/**/node_modules/**/*'
		])
		.pipe(newer('bin'))
		.pipe(scss())
		.pipe(minifyCSS())
		.pipe(gulp.dest('bin'));
});

gulp.task('css', () => {
	return gulp.src([
			'src/**/*.css',
			'!src/**/node_modules/',
			'!src/**/node_modules/**/*'
		])
		.pipe(newer('bin'))
		.pipe(minifyCSS())
		.pipe(gulp.dest('bin'));
});

gulp.task('js', () => {
	return gulp.src([
			'src/**/*.js',
			'!src/**/node_modules/',
			'!src/**/node_modules/**/*'
		])
		.pipe(newer('bin'))
		.pipe(preprocess())
		.pipe(uglify())
		.pipe(minify({
			ext: {
				min: '.js'
			},
			noSource: true
		}))
		.pipe(gulp.dest('bin'));
});

gulp.task('ts', () => {
	return gulp.src([
			'src/**/*.ts',
			'!src/**/node_modules/',
			'!src/**/node_modules/**/*'
		])
		.pipe(newer('bin'))
		.pipe(preprocess())
		.pipe(tsProject())
		.pipe(uglify())
		.pipe(minify({
			ext: {
				min: '.js'
			},
			noSource: true
		}))
		.pipe(gulp.dest('bin'));
});

gulp.task('images', () => {
	return gulp.src([
			'src/**/*.{png,jpg,jpeg,gif,svg,tif}',
			'!src/**/node_modules/',
			'!src/**/node_modules/**/*'
		])
		.pipe(newer('bin'))
		.pipe(gulp.dest('bin'));
});

gulp.task('pretty-data', () => {
	return gulp.src([
			'src/**/*.{xml,json,xlf,svg}',
			'!src/**/node_modules/',
			'!src/**/node_modules/**/*'
		])
		.pipe(newer('bin'))
		.pipe(prettyData({
			type: "minify",
			preserveComments: false
		}))
		.pipe(gulp.dest('bin'));
});

gulp.task('views', () => {
	return gulp.src([
		'src/**/*.{pug,hbs,ejs}',
		'!src/**/node_modules/',
		'!src/**/node_modules/**/*'
	])
	.pipe(newer('bin'))
	.pipe(gulp.dest('bin'));
});

gulp.task('copy', () => {
	return gulp.src([
		'src/**/*',
		'!src/**/*.{pug,hbs,ejs,xml,json,xlf,svg,png,jpg,jpeg,gif,svg,tif,ts,js,css,scss,sass,html}',
		'!src/**/node_modules/',
		'!src/**/node_modules/**/*'
	])
	.pipe(newer('bin'))
	.pipe(gulp.dest('bin'));
});

gulp.task('clean', () => {
	return gulp.src('bin')
		.pipe(clean());
});

gulp.task('default', gulp.parallel('html', 'scss', 'css', 'js', 'ts', 'images', 'pretty-data', 'views', 'copy'));
gulp.task('build', gulp.series('default'));
gulp.task('rebuild', gulp.series('clean', 'default'));