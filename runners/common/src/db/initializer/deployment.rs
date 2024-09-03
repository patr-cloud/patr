use crate::prelude::*;

pub async fn initialize_deployment_tables(
	conn: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up deployment tables");

	query!(
		r#"
		CREATE TABLE deployment_machine_type(
			id TEXT PRIMARY KEY,
			cpu_count INTEGER NOT NULL,
			memory_count INTEGER NOT NULL
		);
		"#
	)
	.execute(&mut *conn)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment(
			id TEXT NOT NULL PRIMARY KEY,
			name TEXT NOT NULL,
			registry TEXT NOT NULL DEFAULT 'docker.io',
			image_name TEXT NOT NULL,
			image_tag TEXT NOT NULL,
			status TEXT 
				CHECK ( status IN (
					'created', 
					'pushed', 
					'deploying', 
					'running', 
					'stopped', 
					'errored', 
					'deleted') 
				) 
				NOT NULL DEFAULT 'created',
			min_horizontal_scale INTEGER NOT NULL DEFAULT 1,
			max_horizontal_scale INTEGER NOT NULL DEFAULT 1,
			machine_type TEXT NOT NULL,
			deploy_on_push BOOLEAN NOT NULL DEFAULT FALSE,
			startup_probe_port INTEGER,
			startup_probe_path TEXT,
			startup_probe_port_type TEXT CHECK(liveness_probe_port_type IN ('tcp', 'http')),
			liveness_probe_port INTEGER,
			liveness_probe_path TEXT,
			liveness_probe_port_type TEXT CHECK (liveness_probe_port_type IN ('tcp', 'http')),
			current_live_digest TEXT
		);
		"#
	)
	.execute(&mut *conn)
	.await?;

	Ok(())
}
