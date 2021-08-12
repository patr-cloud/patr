def main(ctx):
    print(ctx)
    (steps, services) = get_pipeline_steps(ctx)
    return {
        "kind": "pipeline",
        "type": "docker",
        "name": "{} - {} - {} - {}".format(ctx.build.event, ctx.build.target, ctx.build.source, ctx.build.branch),
        "steps": steps,
        "services": services
    }


def get_pipeline_steps(ctx):
    if is_pr(ctx, "develop"):
        return ([
            # Build in debug mode
            build_code(release=False, sqlx_offline=True),
            check_formatting(),  # Check if formatting is fine
            check_clippy(),  # Check clippy lints

            copy_config(),  # Create sample config
            init_database(env=get_app_db_environment()),  # Run --db-only

            clean_api_build(),  # Clean build cache of `api`
            # Run cargo check again, but this time with SQLX_OFFLINE=false
            check_code(release=False, sqlx_offline=False),
        ], [
            database_service(get_database_password())
        ])
    elif is_pr(ctx, "staging"):
        return ([
            # Build in release mode
            build_code(release=True, sqlx_offline=True),
            check_formatting(),  # Check if formatting is fine
            check_clippy(),  # Check clippy lints

            copy_config(),  # Create sample config
            init_database(env=get_app_db_environment()),  # Run --db-only

            clean_api_build(),  # Clean build cache of `api`
            # Run cargo check again, but this time with SQLX_OFFLINE=false
            check_code(release=True, sqlx_offline=False),
        ], [
            database_service(get_database_password())
        ])
    elif is_pr(ctx, "master"):
        return ([
            # Build in release mode
            build_code(release=True, sqlx_offline=True),
            check_formatting(),  # Check if formatting is fine
            check_clippy(),  # Check clippy lints

            copy_config(),  # Create sample config
            init_database(env=get_app_db_environment()),  # Run --db-only

            clean_api_build(),  # Clean build cache of `api`
            # Run cargo check again, but this time with SQLX_OFFLINE=false
            check_code(release=True, sqlx_offline=False),
        ], [
            database_service(get_database_password())
        ])
    elif is_push(ctx, "develop"):
        return ([
            # Build in debug mode
            build_code(release=False, sqlx_offline=True),

            copy_config(),  # Create sample config
            init_database(env=get_app_db_environment()),  # Run --db-only

            clean_api_build(),  # Clean build cache of `api`
            # Run cargo check again, but this time with SQLX_OFFLINE=false
            check_code(release=False, sqlx_offline=False),
        ], [
            database_service(get_database_password())
        ])
    elif is_push(ctx, "staging"):
        return ([
            # Build in release mode
            build_code(release=True, sqlx_offline=True),

            copy_config(),  # Create sample config
            init_database(env=get_app_db_environment()),  # Run --db-only

            clean_api_build(),  # Clean build cache of `api`
            # Run cargo check again, but this time with SQLX_OFFLINE=false
            check_code(release=True, sqlx_offline=False),

            # TODO Deploy
        ], [
            database_service(get_database_password())
        ])
    elif is_push(ctx, "master"):
        return ([
            # Build in release mode
            build_code(release=True, sqlx_offline=True),

            copy_config(),  # Create sample config
            init_database(env=get_app_db_environment()),  # Run --db-only

            clean_api_build(),  # Clean build cache of `api`
            # Run cargo check again, but this time with SQLX_OFFLINE=false
            check_code(release=True, sqlx_offline=False),

            # TODO Deploy
        ], [
            database_service(get_database_password())
        ])
    else:
        return ([], [])


def is_pr(ctx, to_branch):
    return ctx.build.event == "pull_request" and ctx.build.target == to_branch


def is_push(ctx, on_branch):
    return ctx.build.event == "push" and ctx.build.branch == on_branch


def build_code(release, sqlx_offline):
    offline = "false"
    if sqlx_offline == True:
        offline = "true"
    else:
        offline = "false"

    release_flag = ""
    if release == True:
        release_flag = "--release"

    return {
        "name": "Build project",
        "image": "rust:1",
        "commands": [
            "cargo build {}".format(release_flag)
        ],
        "environment": {
            "SQLX_OFFLINE": offline,
            "DATABASE_URL": "postgres://postgres:{}@database:5432/api".format(
                get_database_password())
        }
    }


def check_formatting():
    return {
        "name": "Check code formatting",
        "image": "rustlang/rust:nightly",
        "commands": [
            "cargo fmt -- --check"
        ]
    }


def check_clippy():
    return {
        "name": "Check clippy suggestions",
        "image": "rustlang/rust:nightly",
        "commands": [
            "cargo clippy -- -D warnings"
        ]
    }


def copy_config():
    return {
        "name": "Copy sample config",
        "image": "rust:1",
        "commands": [
            "cp config/dev.sample.json config/dev.json",
            "cp config/dev.sample.json config/prod.json"
        ]
    }


def init_database(env):
    return {
        "name": "Initialize database",
        "image": "rust:1",
        "commands": [
            "cargo run -- --db-only"
        ],
        "environment": env
    }


def clean_api_build():
    return {
        "name": "Clean up build cache",
        "image": "rust:1",
        "commands": [
            "cargo clean -p api"
        ]
    }


def check_code(release, sqlx_offline):
    offline = "false"
    if sqlx_offline == True:
        offline = "true"
    else:
        offline = "false"

    release_flag = ""
    if release == True:
        release_flag = "--release"

    return {
        "name": "Build project",
        "image": "rust:1",
        "commands": [
            "cargo check {}".format(release_flag)
        ],
        "environment": {
            "SQLX_OFFLINE": offline,
            "DATABASE_URL": "postgres://postgres:{}@database:5432/api".format(
                get_database_password())
        }
    }


def database_service(pwd):
    return {
        "name": "database",
        "image": "postgres",
        "environment": {
            "POSTGRES_PASSWORD": pwd,
            "POSTGRES_DB": "api"
        }
    }


def get_database_password():
    return "dAtAbAsEpAsSwOrD"


def get_app_db_environment():
    return {
        "APP_DATABASE_HOST": "database",
        "APP_DATABASE_PORT": 3306,
        "APP_DATABASE_USER": "postgres",
        "APP_DATABASE_PASSWORD": get_database_password(),
        "APP_DATABASE_DATABASE": "api"
    }
