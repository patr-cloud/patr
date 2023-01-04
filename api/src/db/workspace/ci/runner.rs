use api_macros::{query, query_as};
use api_models::{models::workspace::ci::runner::Runner, utils::Uuid};

use crate::Database;

pub async fn initialize_ci_runner_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing ci runner tables");

	query!(
		r#"
		CREATE TABLE ci_runner (
			id              UUID        	NOT NULL,
            name            CITEXT      	NOT NULL,
            workspace_id    UUID        	NOT NULL,
            region_id       UUID        	NOT NULL,
            cpu             INTEGER     	NOT NULL, /* Multiples of ¼th vCPU */
            ram             INTEGER     	NOT NULL, /* Multiples of ¼th GB RAM */
            volume          INTEGER     	NOT NULL, /* Multiples of ¼th GB storage */
			deleted			TIMESTAMPTZ,

            CONSTRAINT ci_runner_pk PRIMARY KEY (id),

            CONSTRAINT ci_runner_fk_workspace_id
                FOREIGN KEY (workspace_id) REFERENCES workspace(id),
            CONSTRAINT ci_runner_fk_region_id
                FOREIGN KEY (region_id) REFERENCES deployment_region(id),

            CONSTRAINT ci_runner_chk_name CHECK(name = TRIM(name)),
            CONSTRAINT ci_runner_chk_cpu CHECK (cpu > 0),
            CONSTRAINT ci_runner_chk_ram CHECK (ram > 0),
            CONSTRAINT ci_runner_chk_volume CHECK (volume > 0)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX ci_runner_uq_workspace_id_name
            ON ci_runner (workspace_id, name)
                WHERE deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE OR REPLACE FUNCTION ci_runner_chk_region_id_workspace_id ()
		RETURNS TRIGGER LANGUAGE PLPGSQL STABLE
		AS $$
		DECLARE
			is_valid BOOLEAN := FALSE;
		BEGIN
			SELECT TRUE INTO is_valid
			FROM deployment_region
			WHERE
				deployment_region.id = NEW.region_id
				AND (
					deployment_region.workspace_id IS NULL
					OR deployment_region.workspace_id = NEW.workspace_id
				);
		
			IF is_valid THEN
				RETURN NEW;
			ELSE
				RAISE EXCEPTION 'workspace does not have given region';
			END IF;
		END
		$$;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TRIGGER ci_runner_trg_chk_region_id_workspace_id
			BEFORE INSERT OR UPDATE ON ci_runner
			FOR EACH ROW EXECUTE FUNCTION ci_runner_chk_region_id_workspace_id();
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_ci_runner_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up ci tables initialization");

	query!(
		r#"
		ALTER TABLE ci_runner
        ADD CONSTRAINT ci_runner_fk_id_workspace_id
        FOREIGN KEY(id, workspace_id) REFERENCES resource(id, owner_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn get_runners_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Vec<Runner>, sqlx::Error> {
	query_as!(
		Runner,
		r#"
			SELECT
				id as "id: _",
				name as "name: _",
				workspace_id as "workspace_id: _",
				region_id as "region_id: _",
				cpu,
				ram,
				volume
			FROM ci_runner
			WHERE
				workspace_id = $1 AND
				deleted IS NULL;
		"#,
		workspace_id as _
	)
	.fetch_all(connection)
	.await
}

pub async fn create_runner_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	runner_id: &Uuid,
	name: &str,
	workspace_id: &Uuid,
	region_id: &Uuid,
	cpu: i32,
	ram: i32,
	volume: i32,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO ci_runner (
			id,
			name,
			workspace_id,
			region_id,
			cpu,
			ram,
			volume
		)
		VALUES ($1, $2, $3, $4, $5, $6, $7);
		"#,
		runner_id as _,
		name as _,
		workspace_id as _,
		region_id as _,
		cpu,
		ram,
		volume,
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn get_runner_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	runner_id: &Uuid,
) -> Result<Option<Runner>, sqlx::Error> {
	query_as!(
		Runner,
		r#"
			SELECT
				id as "id: _",
				name as "name: _",
				workspace_id as "workspace_id: _",
				region_id as "region_id: _",
				cpu,
				ram,
				volume
			FROM ci_runner
			WHERE
				id = $1 AND
				deleted IS NULL;
		"#,
		runner_id as _,
	)
	.fetch_optional(connection)
	.await
}

pub async fn update_runner(
	connection: &mut <Database as sqlx::Database>::Connection,
	runner_id: &Uuid,
	name: &str,
) -> Result<(), sqlx::Error> {
	query_as!(
		Runner,
		r#"
			UPDATE ci_runner
			SET name = $2
			WHERE
				id = $1 AND
				deleted IS NULL;
		"#,
		runner_id as _,
		name as _,
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn mark_runner_as_deleted(
	connection: &mut <Database as sqlx::Database>::Connection,
	runner_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query_as!(
		Runner,
		r#"
			UPDATE ci_runner
			SET deleted = NOW()
			WHERE
				id = $1 AND
				deleted IS NULL;
		"#,
		runner_id as _,
	)
	.execute(connection)
	.await
	.map(|_| ())
}
