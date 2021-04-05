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
        "image": "ubuntu:latest",
        "command": [
            "apt update",
            "apt install -y curl",
            "curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly",
            "cargo +nightly check"
        ]
    }

def check_formatting():
    return {
        "name": "Check code formatting",
        "image": "ubuntu:latest",
        "command": [
            "apt update",
            "apt install -y curl",
            "curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly",
            "cargo +nightly fmt -- --check"
        ]
    }

def check_clippy():
    return {
        "name": "Check clippy suggestions",
        "image": "ubuntu:latest",
        "command": [
            "apt update",
            "apt install -y curl",
            "curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly",
            "cargo +nightly clippy"
        ]
    }
