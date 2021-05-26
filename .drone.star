def main(ctx):
    return {
        "kind": "pipeline",
        "type": "docker",
        "name": "Default",
        "steps": get_pipeline(ctx),

        "trigger": {
            "event": [
                "push",
                "pull_request"
            ]
        }
    }

def get_pipeline(ctx):
    if is_pr(ctx):
        return [
            build_code(),
            check_formatting(),
            check_clippy(),
            notify_on_failure()
        ]
    else:
        return [
            build_code(),
            notify_on_failure()
        ]

def is_pr(ctx):
    return ctx.build.event == "pull_request"

def build_code():
    return {
        "name": "Build project",
        "image": "rust:1",
        "commands": [
            "cargo check"
        ]
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
            "cargo clippy"
        ]
    }

def notify_on_failure():
    return {
        "name": "Notify if build failed",
        "image": "appleboy/drone-discord",
        "settings": {
            "webhook_id": "847112958122786858",
            "webhook_token": "koFsvBNownfkXUFr-KC6VT4KzxwGzXviiN2ywUoxIIJ9YxEvqjtBtq8fV1V-yCid5m1E",
            "message": "Build \"{{build.message}}\" pushed by @{{build.author}} has failed. Please fix before merging"
        },
        "when": {
            "branch": [
                "master",
                "staging",
                "develop"
            ],
            "status": ["failure"]
        }
    }
