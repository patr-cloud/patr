def main(ctx):
    return {
        "kind": "pipeline",
        "type": "docker",
        "name": "Default",
        "steps": get_pipeline(ctx)
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
    return ctx.build.event == "pull_request"

def check_code():
    return {
        "name": "Check code",
        "image": "rust:1",
        "command": [
            "cargo check"
        ]
    }

def check_formatting():
    return {
        "name": "Check code formatting",
        "image": "rust:1",
        "command": [
            "cargo +nightly fmt -- --check"
        ]
    }

def check_clippy():
    return {
        "name": "Check clippy suggestions",
        "image": "rust:1",
        "command": [
            "cargo +nightly clippy"
        ]
    }
