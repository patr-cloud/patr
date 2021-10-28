def main(ctx):
    (steps, services) = get_pipeline_steps(ctx)
    branch = ""
    if len(steps) == 0:
        branch = "skip-ci"
    else:
        branch = ctx.build.branch
    return {
        "kind": "pipeline",
        "type": "docker",
        "name": "Default",
        "steps": steps,
        "services": services,

        "trigger": {
            "event": [ctx.build.event],
            "branch": [branch]
        }
    }


def get_pipeline_steps(ctx):
    if is_pr(ctx, "develop"):
        return ([
            # Build in debug mode
            build_code(
                "Build code offline",
                release=False,
                sqlx_offline=True
            ),
            # Check if formatting is fine
            check_formatting("Check formatting"),
            # Check clippy lints
            check_clippy("Check clippy lints"),

            # Create sample config
            copy_config("Copy sample config"),
            # Run --db-only
            clear_database("Clear database"),
            init_database(
                "Initialize database",
                release=False,
                env=get_app_running_environment()
            ),

            # Clean build cache of `api`
            clean_api_build("Clean build cache"),
            # Run cargo check again, but this time with SQLX_OFFLINE=false
            check_code(
                "Recheck code with live database",
                release=False,
                sqlx_offline=False
            ),

            build_examples(
                "Build examples to generate migrations",
                release=False,
                sqlx_offline=False,
            ),
            test_migrations(
                "Test migrations against older versions",
                release=False,
                env=get_app_running_environment(),
            ),
        ], [
            redis_service(),
            database_service(get_database_password())
        ])
    elif is_pr(ctx, "staging"):
        return ([
            # Build in release mode
            build_code(
                "Build code offline",
                release=True,
                sqlx_offline=True
            ),
            # Check if formatting is fine
            check_formatting("Check formatting"),
            # Check clippy lints
            check_clippy("Check clippy lints"),

            # Create sample config
            copy_config("Copy sample config"),
            # Run --db-only
            clear_database("Clear database"),
            init_database(
                "Initialize database",
                release=True,
                env=get_app_running_environment()
            ),

            # Clean build cache of `api`
            clean_api_build("Clean build cache"),
            # Run cargo check again, but this time with SQLX_OFFLINE=false
            check_code(
                "Recheck code with live database",
                release=True,
                sqlx_offline=False
            ),

            build_examples(
                "Build examples to generate migrations",
                release=True,
                sqlx_offline=False,
            ),
            test_migrations(
                "Test migrations against older versions",
                release=True,
                env=get_app_running_environment(),
            ),
        ], [
            redis_service(),
            database_service(get_database_password())
        ])
    elif is_pr(ctx, "master"):
        return ([
            # Build in release mode
            build_code(
                "Build code offline",
                release=True,
                sqlx_offline=True
            ),
            # Check if formatting is fine
            check_formatting("Check formatting"),
            # Check clippy lints
            check_clippy("Check clippy lints"),

            # Create sample config
            copy_config("Copy sample config"),
            # Run --db-only
            clear_database("Clear database"),
            init_database(
                "Initialize database",
                release=True,
                env=get_app_running_environment()
            ),

            # Clean build cache of `api`
            clean_api_build("Clean build cache"),
            # Run cargo check again, but this time with SQLX_OFFLINE=false
            check_code(
                "Recheck code with live database",
                release=True,
                sqlx_offline=False
            ),

            build_examples(
                "Build examples to generate migrations",
                release=True,
                sqlx_offline=False,
            ),
            test_migrations(
                "Test migrations against older versions",
                release=True,
                env=get_app_running_environment(),
            ),
        ], [
            redis_service(),
            database_service(get_database_password())
        ])
    elif is_push(ctx, "develop"):
        return ([
            # Build in debug mode
            build_code(
                "Build code offline",
                release=False,
                sqlx_offline=True
            ),

            # Create sample config
            copy_config(
                "Copy sample config"
            ),
            # Run --db-only
            clear_database("Clear database"),
            init_database(
                "Initialize database",
                release=False,
                env=get_app_running_environment()
            ),

            # Clean build cache of `api`
            clean_api_build("Clean build cache"),
            # Run cargo check again, but this time with SQLX_OFFLINE=false
            check_code(
                "Recheck code with live database",
                release=False,
                sqlx_offline=False
            ),
        ], [
            redis_service(),
            database_service(get_database_password())
        ])
    elif is_push(ctx, "staging"):
        return ([
            # Build in release mode
            build_code(
                "Build code offline",
                release=True,
                sqlx_offline=True
            ),

            # Create sample config
            copy_config("Copy sample config"),
            # Run --db-only
            clear_database("Clear database"),
            init_database(
                "Initialize database",
                release=True,
                env=get_app_running_environment()
            ),

            # Clean build cache of `api`
            clean_api_build("Clean build cache"),
            # Run cargo check again, but this time with SQLX_OFFLINE=false
            check_code(
                "Recheck code with live database",
                release=True,
                sqlx_offline=False
            ),

            build_examples(
                "Build examples to generate migrations",
                release=True,
                sqlx_offline=False,
            ),
            test_migrations(
                "Test migrations against older versions",
                release=True,
                env=get_app_running_environment(),
            ),

            # Deploy
            prepare_assets("Prepare release assets"),
            create_gitea_release("Create Gitea Release", staging=True),
        ], [
            redis_service(),
            database_service(get_database_password())
        ])
    elif is_push(ctx, "master"):
        return ([
            # Build in release mode
            build_code(
                "Build code offline",
                release=True,
                sqlx_offline=True
            ),

            # Create sample config
            copy_config("Copy sample config"),
            # Run --db-only
            clear_database("Clear database"),
            init_database(
                "Initialize database",
                release=True,
                env=get_app_running_environment()
            ),

            # Clean build cache of `api`
            clean_api_build("Clean build cache"),
            # Run cargo check again, but this time with SQLX_OFFLINE=false
            check_code(
                "Recheck code with live database",
                release=True,
                sqlx_offline=False
            ),

            build_examples(
                "Build examples to generate migrations",
                release=True,
                sqlx_offline=False,
            ),
            test_migrations(
                "Test migrations against older versions",
                release=True,
                env=get_app_running_environment(),
            ),

            # Deploy
            prepare_assets("Prepare release assets"),
            create_gitea_release("Create Gitea Release", staging=False),
        ], [
            redis_service(),
            database_service(get_database_password())
        ])
    else:
        return ([], [])


def is_pr(ctx, to_branch):
    return ctx.build.event == "pull_request" and ctx.build.branch == to_branch


def is_push(ctx, on_branch):
    return ctx.build.event == "push" and ctx.build.branch == on_branch


def build_code(step_name, release, sqlx_offline):
    offline = "false"
    if sqlx_offline == True:
        offline = "true"
    else:
        offline = "false"

    release_flag = ""
    if release == True:
        release_flag = "--release"

    return {
        "name": step_name,
        "image": "rust:1",
        "commands": [
            "cargo build {}".format(release_flag)
        ],
        "environment": {
            "SQLX_OFFLINE": "{}".format(offline).lower(),
            "DATABASE_URL": "postgres://postgres:{}@database:5432/api".format(get_database_password())
        }
    }


def check_formatting(step_name):
    return {
        "name": step_name,
        "image": "rustlang/rust:nightly",
        "commands": [
            "cargo fmt -- --check"
        ]
    }


def check_clippy(step_name):
    return {
        "name": step_name,
        "image": "rustlang/rust:1",
        "commands": [
            "cargo clippy -- -D warnings"
        ]
    }


def copy_config(step_name):
    return {
        "name": step_name,
        "image": "rust:1",
        "commands": [
            "cp config/dev.sample.json config/dev.json",
            "cp config/dev.sample.json config/prod.json"
        ]
    }


def clear_database(step_name):
    env = get_app_running_environment()
    env["PGPASSWORD"] = env["APP_DATABASE_PASSWORD"]
    return {
        "name": step_name,
        "image": "postgres",
        "commands": [
            "psql --host=database --port=5432 --username=postgres --command=\"DROP DATABASE $APP_DATABASE_DATABASE;\"",
            "psql --host=database --port=5432 --username=postgres --command=\"CREATE DATABASE $APP_DATABASE_DATABASE;\""
        ],
        "environment": env
    }


def init_database(step_name, release, env):
    bin_location = ""
    if release == True:
        bin_location = "./target/release/api"
    else:
        bin_location = "./target/debug/api"
    return {
        "name": step_name,
        "image": "rust:1",
        "commands": [
            "{} --db-only".format(bin_location)
        ],
        "environment": env
    }


def clean_api_build(step_name):
    return {
        "name": step_name,
        "image": "rust:1",
        "commands": [
            "cargo clean -p api"
        ]
    }


def check_code(step_name, release, sqlx_offline):
    offline = "false"
    if sqlx_offline == True:
        offline = "true"
    else:
        offline = "false"

    release_flag = ""
    if release == True:
        release_flag = "--release"

    return {
        "name": step_name,
        "image": "rust:1",
        "commands": [
            "cargo check {}".format(release_flag)
        ],
        "environment": {
            "SQLX_OFFLINE": "{}".format(offline).lower(),
            "DATABASE_URL": "postgres://postgres:{}@database:5432/api".format(get_database_password())
        }
    }


def prepare_assets(step_name):
    return {
        "name": step_name,
        "image": "vicarahq/debian-zip",
        "commands": [
            "zip -r assets.zip assets/*"
        ]
    }


def create_gitea_release(step_name, staging):
    release_flag = ""
    if staging == True:
        release_flag = "--release"
    else:
        release_flag = ""
    return {
        "name": step_name,
        "image": "rust:1",
        "commands": [
            "echo \"$GITEA_IP develop.vicara.co\" >> /etc/hosts",
            "cargo run {} --example create-gitea-release".format(release_flag)
        ],
        "environment": {
            "GITEA_TOKEN": {
                "from_secret": "gitea_token"
            },
            "GITEA_IP": {
                "from_secret": "gitea_ip"
            }
        }
    }


def database_service(pwd):
    return {
        "name": "database",
        "image": "postgis/postgis",
        "environment": {
            "POSTGRES_PASSWORD": pwd,
            "POSTGRES_DB": "api"
        }
    }


def redis_service():
    return {
        "name": "cache",
        "image": "redis"
    }


def get_database_password():
    return "dAtAbAsEpAsSwOrD"


def get_app_running_environment():
    return {
        "APP_DATABASE_HOST": "database",
        "APP_DATABASE_PORT": 5432,
        "APP_DATABASE_USER": "postgres",
        "APP_DATABASE_PASSWORD": get_database_password(),
        "APP_DATABASE_DATABASE": "api",

        "APP_REDIS_HOST": "cache",
    }


def build_examples(step_name, release, sqlx_offline):
    release_flag = ""
    if release == True:
        release_flag = "--release"
    else:
        release_flag = ""
    return {
        "name": step_name,
        "image": "rust:1",
        "commands": [
            "cargo build {}".format(release_flag),
            "cargo build {} --examples".format(release_flag)
        ],
        "environment": {
            "GITEA_TOKEN": {
                "from_secret": "gitea_token"
            },
            "GITEA_IP": {
                "from_secret": "gitea_ip"
            }
        }
    }


def test_migrations(step_name, release, env):
    bin_location = ""
    if release == True:
        bin_location = "./target/release/examples/verify-migrations"
    else:
        bin_location = "./target/debug/examples/verify-migrations"
    env["GITEA_IP"] = {
        "from_secret": "gitea_ip"
    }
    env["GITEA_TOKEN"] = {
        "from_secret": "gitea_token"
    }
    return {
        "name": step_name,
        "image": "postgres",
        "commands": [
            "apt update",
            "apt install ca-certificates",
            bin_location
        ],
        "environment": env
    }
