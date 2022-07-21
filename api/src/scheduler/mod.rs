use std::{future::Future, pin::Pin, time::Duration};

use chrono::Utc;
use cron::Schedule;
use once_cell::sync::OnceCell;
use tokio::{task, time};

use crate::{app::App, utils::Error};

static CONFIG: OnceCell<App> = OnceCell::new();

pub mod billing;
pub mod domain;
pub mod syncr;

pub fn initialize_jobs(app: &App) {
	CONFIG.set(app.clone()).expect("CONFIG is already set");

	let jobs = get_scheduled_jobs();

	for job in jobs {
		task::spawn(run_job(job));
	}
}

async fn run_job(job: Job) {
	let mut last_tick = Utc::now();
	loop {
		let now = Utc::now();
		if let Some(event) = job.schedule.after(&last_tick).next() {
			if event > now {
				time::sleep(Duration::from_millis(
					(event - now).num_milliseconds().unsigned_abs(),
				))
				.await;
				continue;
			}
			last_tick = now;
			let result = (job.runner)().await;
			if let Err(err) = result {
				log::error!(
					"Error while trying to run job `{}`: {}",
					job.name,
					err.get_error()
				);
			}
		}
	}
}

fn get_scheduled_jobs() -> Vec<Job> {
	vec![
		domain::verify_unverified_domains_job(),
		domain::repatch_all_managed_urls_job(),
		domain::reverify_verified_domains_job(),
		domain::refresh_domain_tld_list_job(),
		billing::update_bill_job(),
		syncr::initiate_syncr_job(),
	]
}

type JobRunner =
	fn() -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>;

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
