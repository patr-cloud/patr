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
            check_code(),
            check_formatting(),
            check_clippy()
        ]
    else:
        return []

def is_pr(ctx):
    return (ctx.build.event == "pull_request") or (ctx.build.event == "push")

def check_code():
    return {
        "name": "Check code",
        "image": "rust:1",
        "command": [
            "rustup target add x86_64-unknown-linux-gnu",
            "rustup toolchain install nightly",
            "cargo check"
        ]
    }

def check_formatting():
    return {
        "name": "Check code formatting",
        "image": "rust:1",
        "command": [
            "rustup target add x86_64-unknown-linux-gnu",
            "rustup toolchain install nightly",
            "cargo +nightly fmt -- --check"
        ]
    }

def check_clippy():
    return {
        "name": "Check clippy suggestions",
        "image": "rust:1",
        "command": [
            "rustup target add x86_64-unknown-linux-gnu",
            "rustup toolchain install nightly",
            "cargo +nightly clippy"
        ]
    }
