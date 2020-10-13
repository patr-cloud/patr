use crate::{
	utils::settings::{RunningEnvironment, Settings},
	Result,
};

use log::LevelFilter;
use log4rs::{
	append::{
		console::ConsoleAppender,
		rolling_file::{
			policy::compound::{
				roll::fixed_window::FixedWindowRoller,
				trigger::size::SizeTrigger, CompoundPolicy,
			},
			RollingFileAppender,
		},
	},
	config::{Appender, Config, Logger, Root},
	encode::pattern::PatternEncoder,
	filter::threshold::ThresholdFilter,
	Handle,
};

pub async fn initialize(config: &Settings) -> Result<Handle> {
	println!("[TRACE]: Initializing logger...");
	let config = match config.environment {
		RunningEnvironment::Development => Config::builder()
			.appender(
				Appender::builder()
					.filter(Box::new(ThresholdFilter::new(LevelFilter::Error)))
					.build(
						"default",
						Box::new(
							ConsoleAppender::builder()
								.encoder(Box::new(PatternEncoder::new("")))
								.build(),
						),
					),
			)
			.appender(
				Appender::builder().build(
					"console",
					Box::new(
						ConsoleAppender::builder()
							.encoder(Box::new(PatternEncoder::new("[{h({l})}]: {m}{n}")))
							.build(),
					),
				),
			)
			.logger(
				Logger::builder()
					.appender("default")
					.additive(false)
					.build("api::queries", LevelFilter::Trace),
			)
			.logger(
				Logger::builder()
					.appender("console")
					.build("api", LevelFilter::Trace),
			)
			.build(Root::builder().appender("default").build(LevelFilter::Warn))?,
		RunningEnvironment::Production => Config::builder()
			.appender(
				Appender::builder().build(
					"default",
					Box::new(
						RollingFileAppender::builder()
							.encoder(Box::new(PatternEncoder::new(
								"[{d(%a, %d-%b-%Y %I:%M:%S %P)} - {l}]: {m}{n}",
							)))
							.append(true)
							.build(
								"log/internals.log",
								Box::new(CompoundPolicy::new(
									Box::new(SizeTrigger::new(1024 * 1024 * 100)),
									Box::new(
										FixedWindowRoller::builder()
											.base(0)
											.build("log/internals.{}.gz", 10)
											.expect("unable to build fixed window roller"),
									),
								)),
							)?,
					),
				),
			)
			.appender(
				Appender::builder()
					.filter(Box::new(ThresholdFilter::new(LevelFilter::Info)))
					.build(
						"console",
						Box::new(
							ConsoleAppender::builder()
								.encoder(Box::new(PatternEncoder::new("[{h({l})}]: {m}{n}")))
								.build(),
						),
					),
			)
			.appender(
				Appender::builder().build(
					"requests",
					Box::new(
						RollingFileAppender::builder()
							.encoder(Box::new(PatternEncoder::new(
								"[{d(%a, %d-%b-%Y %I:%M:%S %P)} - {l}]: {m}{n}",
							)))
							.append(true)
							.build(
								"log/app.log",
								Box::new(CompoundPolicy::new(
									Box::new(SizeTrigger::new(1024 * 1024 * 100)),
									Box::new(
										FixedWindowRoller::builder()
											.base(0)
											.build("log/app.{}.gz", 10)
											.expect("unable to build fixed window roller"),
									),
								)),
							)?,
					),
				),
			)
			.appender(
				Appender::builder().build(
					"queries",
					Box::new(
						RollingFileAppender::builder()
							.encoder(Box::new(PatternEncoder::new(
								"[{d(%a, %d-%b-%Y %I:%M:%S %P)}]: {m}{n}",
							)))
							.append(true)
							.build(
								"log/queries.log",
								Box::new(CompoundPolicy::new(
									Box::new(SizeTrigger::new(1024 * 1024 * 50)),
									Box::new(
										FixedWindowRoller::builder()
											.base(0)
											.build("log/queries.{}.gz", 10)
											.expect("unable to build fixed window roller"),
									),
								)),
							)?,
					),
				),
			)
			.logger(
				Logger::builder()
					.appender("console")
					.appender("requests")
					.additive(false)
					.build("api", LevelFilter::Trace),
			)
			.logger(
				Logger::builder()
					.appender("queries")
					.additive(false)
					.build("api::queries", LevelFilter::Trace),
			)
			.build(
				Root::builder()
					.appender("default")
					.build(LevelFilter::Trace),
			)?,
	};

	Ok(log4rs::init_config(config)?)
}
