use std::{future::Future, pin::Pin, time::Duration};

use chrono::Utc;
use cron::Schedule;
use once_cell::sync::OnceCell;
use tokio::{task, time};

use crate::app::App;

static CONFIG: OnceCell<App> = OnceCell::new();

pub mod domain;

pub fn initialize_jobs(app: &App) {
	CONFIG.set(app.clone()).expect("CONFIG is already set");

	let jobs = get_scheduled_jobs();

	for job in jobs {
		task::spawn(run_job(job));
	}
}

async fn run_job(job: Job) {
	let mut last_tick = None;
	loop {
		let now = Utc::now();
		if last_tick.is_none() {
			last_tick = Some(now);
			continue;
		}
		if let Some(event) =
			job.schedule.after(last_tick.as_ref().unwrap()).next()
		{
			if event > now {
				time::delay_for(Duration::from_millis(
					(event - now).num_milliseconds().abs() as u64,
				))
				.await;
				continue;
			}
			last_tick = Some(now);
			let result = (job.runner)().await;
			if let Err(err) = result {
				log::error!(
					"Error while trying to run job `{}`: {}",
					job.name,
					err
				);
			}
		}
	}
}

fn get_scheduled_jobs() -> Vec<Job> {
	vec![
		domain::verify_unverified_domains_job(),
		domain::reverify_verified_domains_job(),
	]
}

type JobRunner =
	fn() -> Pin<Box<dyn Future<Output = crate::Result<()>> + Send>>;

struct Job {
	name: String,
	schedule: Schedule,
	runner: JobRunner,
}

impl Job {
	fn new(name: String, schedule: Schedule, runner: JobRunner) -> Self {
		Job {
			name,
			schedule,
			runner,
		}
	}
}
