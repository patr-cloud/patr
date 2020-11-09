use job_scheduler::Job;
use once_cell::sync::OnceCell;

use crate::app::App;

pub static CONFIG: OnceCell<App> = OnceCell::new();

pub mod domain;

pub fn get_scheduled_jobs<'a>() -> Vec<Job<'a>> {
	vec![
		domain::verify_unverified_domains_job(),
		domain::reverify_verified_domains_job(),
	]
}
