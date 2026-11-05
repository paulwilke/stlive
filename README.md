# stlive — live Pharo development from the command line

Inspect a running **Pharo Smalltalk** image, keep a failed execution available for debugging, compile a fix, and save the source changes to Git.

ST Live is for developers and coding agents that work through a shell. The short-lived Rust client talks to a long-lived Pharo image; objects and debugger sessions stay in Pharo between commands.

## Install and try it

You need a Rust toolchain, plus `curl` and `unzip` for automatic setup. Setup downloads Pharo 14 (or 13 with `stlive init --pharo 13`); one binary supports both. macOS and Linux are the primary platforms; core commands have also been verified on Windows 11. See [platform support](docs/operations.md#platform-support).

```sh
cargo install --git https://github.com/paulwilke/stlive --locked
stlive init
```

`init` builds a reusable server template under `~/.stlive`; it does not start a project instance. To use an existing local Pharo installation instead:

```sh
stlive init --vm /absolute/path/to/Pharo --image /absolute/path/to/Pharo.image
```

`init` uses Pharo 14 on a fresh home; `stlive init --pharo 13` installs Pharo 13 instead. If the home already has a template of another version, `init --pharo N` installs N beside it (the existing default stays), and `stlive start --pharo N` / `stlive run --pharo N` use it. With `--vm/--image` the version of the given image is detected. Homes made by earlier releases (Pharo 13, no version recorded) keep working unchanged.

Then, in your project directory:

```sh
stlive start
stlive eval '3 + 4'
```

The evaluation returns JSON with `ok: true` and `result.value.print: "7"`. Project state lives in `./.stlive`, with its own `.gitignore`.

**Start here:** [a complete crash → inspect → fix → test → save tutorial](docs/getting-started.md). It creates its own example classes; no existing application is required. Shell examples use Bash-compatible syntax; the tutorial uses `jq` to extract IDs.

## What it does

| Task | Example |
|---|---|
| Evaluate in the running image | `stlive eval '3 + 4'` |
| Read source or discover an API | `stlive method show 'Cart>>total'`, `stlive find implementors total` |
| Inspect a failed execution | `stlive debug frames SESSION --limit 5`, `stlive debug frame SESSION 3` |
| Evaluate with a frame's variables | `stlive debug eval SESSION --frame 3 'subtotal'` |
| Follow a live object | `stlive obj show REF`, `stlive obj items REF --limit 10` |
| Compile a method from a file | `stlive method compile Cart --file total.st` |
| Repeat the original operation | `stlive debug rerun SESSION` |
| Run tests, grouped by failure cause | `stlive test run 'MyApp-Tests'` |
| Save source changes and check agreement | `stlive save --package MyApp --dir src --verify` |
| Test saved baseline sources separately | `stlive test run --fresh --load MyApp --repository "tonel://$PWD/src" --group Tests` |
| Run a program from Tonel files | `stlive run --load src --eval 'MyApp run' -- arg1` |
| Work with a Spec2/Morphic window | `stlive start --gui`, `stlive ui windows`, `stlive ui screenshot` |

`Cart`, `MyApp`, `SESSION` and `REF` stand for your application's names and returned IDs. Use `stlive <command> --help` or the [command reference](docs/commands.md).

For simple batch work, Pharo's built-in evaluation, scripts and test runner may be enough. ST Live adds persistent object references and preserved failure sessions when running state matters.

## What each check proves

| Operation | Meaning |
|---|---|
| `debug resume` | Continue the suspended execution, including frames that may use old methods. |
| `debug restart --frame N` | Restart one frame; surrounding object state is retained. |
| `debug rerun` | Execute the original operation again in the same image. Earlier side effects remain. |
| `save --verify` / `drift` | Compare live code with Tonel files; no behavioral tests are run. |
| `test run` | Run selected tests in the current image. Check targets, counts and skips. |
| `test run --fresh` | Load a baseline and run tests in a temporary image based on the installed template. Save first to test your changes. |

`fresh_execution: true` means a new execution, not a new image or verified repair. Successful evaluation alone does not prove that its value is correct. See [project workflows](docs/workflows.md).

## Use with a coding agent

The [agent skill](skill/SKILL.md) starts with a compact working loop and loads detail when needed. Copy the **whole directory**, including references, into your agent's skill location. For Claude Code, from a checkout of this repository:

```sh
mkdir -p ~/.claude/skills/stlive
cp -R skill/. ~/.claude/skills/stlive/
```

Other shell-capable agents can read the same instructions. Guidance does not guarantee model reliability. This repository ships a CLI, not an MCP server; see the [CLI/MCP comparison](docs/cli-vs-mcp.md).

## State and local access

- `save` writes source files; `image save` snapshots the image. They preserve different things.
- `stop --force` discards image state not snapshotted. Save source changes before stopping; see [storage and lifecycle](docs/operations.md#storage-and-lifecycle).
- Reloading a baseline lets files overwrite modified packages in the image. Save live changes first if you want to retain them.
- The server accepts arbitrary Smalltalk on loopback, **without authentication**. Keep it local; use `image export` to remove it from an exported application image.
- `attach` needs an existing compatible JSON-line evaluation endpoint. It cannot attach to an arbitrary Pharo process by PID. See [attaching](docs/operations.md#attach-to-an-existing-image).

## Documentation

| Read this | For |
|---|---|
| [Getting started](docs/getting-started.md) | Two runnable examples in a disposable project |
| [Project workflows](docs/workflows.md) | Baselines, source round trips, fresh tests and GUI work |
| [Operations and troubleshooting](docs/operations.md) | Directories, instances, exit codes, compatibility and recovery |
| [Command reference](docs/commands.md) | Generated command and option help |
| [Architecture](docs/architecture.md) | Protocol, workers, sessions and implementation |
| [Agent skill](skill/SKILL.md) | Compact guidance for coding agents |
| [Changelog](CHANGELOG.md) | Version-specific changes and historical validation |

Application examples: [paulwilke/stlive-examples](https://github.com/paulwilke/stlive-examples).

## Development

```sh
cargo build --release --locked
cargo test --release --locked
# Requires an initialized template and jq; starts a temporary instance:
./tests/e2e.sh
# Regenerate CLI help after changing commands:
./scripts/gen-reference.sh
```

Rust client code is in `src/`; the Pharo server and SUnit tests are in `smalltalk/src/`. Smalltalk sources are embedded in the binary and loaded during setup. See [architecture](docs/architecture.md) and [validation](docs/operations.md#validation-and-known-limitations).

## License

MIT; see [LICENSE](LICENSE). Setup fetches Pharo and NeoJSON from upstream; exported applications containing them must retain their MIT notices. See [third-party software](docs/operations.md#third-party-software).

ST Live is independent of the Pharo project and consortium. Product names identify supported software and do not imply endorsement.
