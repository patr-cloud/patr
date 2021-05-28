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
            notify_on_failure(ctx)
        ]
    else:
        return [
            build_code(),
            notify_on_failure(ctx)
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

def notify_on_failure(ctx):
    return {
        "name": "Notify if build failed",
        "image": "appleboy/drone-discord",
        "settings": {
            "webhook_id": {
                "from_secret": "webhook_id"
            },
            "webhook_token": {
                "from_secret": "webhook_token"
            },
            "message": """
**Build failed**
----------------

**Commit message**
```
{{commit.message}}
```

**Author**
%s

Please fix before merging
""".format(get_author_list())
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

def get_author_list():
    code = ""
    authors = {
        "abhishek": "427846410101325825",
        "tsgowtham": "328247582835081237",
        "rakshith": "455822434919120926",
        "manjeet.arneja": "434292143507374080",
        "aniket.jain": "764032015041036320",
        "samyak.gangwal": "429563803315994624",
        "satyam.jha": "417720780835782657",
        "rohit.singh": "688020346451918929",
        "sanskar.biswal": "688020277829173337"
    }
    for author in authors:
        code += "{{{{#equal commit.author \"{}\"}}}}<@{}>{{{{/equal}}}}".format(author, authors[author])
    return code