---
name: stlive
description: Work in a running Pharo 13/14 (Smalltalk) image from the shell with the `stlive` CLI – evaluate code and look things up, reload a Metacello baseline from Tonel files, run SUnit tests (also in a fresh image), write live changes back to Tonel with conflict and drift checks, and debug a failure from its kept stack. Use for Pharo/Smalltalk tasks where running code matters: checking behaviour, failing tests, reloading edited Tonel files, unknown library internals.
---

# stlive

Commands print JSON: `{"ok":true,"result":…}` or `{"ok":false,"error":…}`. An error's `message` and `hint` usually name the next step (`did_you_mean`, compile `line`/`column`, `would_lose`, where an instance runs). Read them and change the input; never resend an unchanged failed call. Exact options: `stlive <command> --help`.

Project conventions come first: when CLAUDE.md, AGENTS.md or the task names a reload/test script, an instance name or a state directory, use those; the commands below are what such scripts wrap.

## Which image

- Use the project's running instance; never stop, restart or reload an image someone else works in. `stlive list` shows the instances, `stlive status` the one you address.
- Several agents: one instance each, named on **every** call with `stlive -i NAME …` (shell variables do not survive between tool calls). `STLIVE_DIR` / `--state-dir` selects the state directory. `not_running` names the directory where an instance of that name does run: fix the name or path rather than starting a second image.
- `stop` stops only an instance with nothing to lose. Otherwise read `error.would_lose`, save or terminate what matters, then `stop --force` – only in an instance that is yours.

## Evaluate and look up

```sh
stlive eval --no-session 'Cart new total'
stlive eval --no-session <<'EOF'
(Cart new add: (Item price: 3); yourself) total = 3
EOF
stlive eval --file probe.st
STLIVE_PRINT_LIMIT=2000 stlive eval --no-session 'Cart allSelectors'
stlive method show 'Cart>>total'
stlive class show Cart
stlive find implementors total      # also: find senders, find class 'Cart*'
```

- Code with several lines or string literals goes through a quoted heredoc (`<<'EOF'`) or `--file`, never inside a single-quoted shell argument.
- `result.value.print` is cut at 120 characters (`print_truncated`). Raise it with `STLIVE_PRINT_LIMIT` / `--print-limit`, or use `--full` (whole text in `result.text`).
- **Let errors signal.** Do not wrap code in `on: Error do: [ :e | e ]` to see an error: stlive already reports `type`, `message` and the top frame. An exception answered as a value gives `ok: true` with `result.value_is_exception: true` – that is a failure.
- Use `--no-session` for exploration, so a failed snippet leaves no suspended session.
- End a check with a comparison (`… total = 3`); an expression that completes proves nothing.
- `Cart>>total` is instance side, `'Cart class>>new'` class side. Read real selectors before calling them; an instance variable has no getter unless one is defined.
- Define classes with `stlive class create Foo --superclass Object --ivars 'a b' --package Pkg` or `Object << #Foo slots: { #a. #b }; package: 'Pkg'; install`. `subclass:instanceVariableNames:…` does not exist on Pharo 13/14.

## Change code: one path per change

**Files first** – edit the Tonel files, then reload (or run the project's own reload script):

```sh
stlive load MyApp --repository "tonel://$PWD/src" --group Tests --protect-unsaved
stlive test run 'MyApp-Tests'
```

`load` lets the files win: unsaved live changes in the reloaded packages are overwritten (`modified_packages_overwritten`). With `--protect-unsaved` it refuses instead (`unsaved_changes`, nothing loaded). A load can exit 0 and still list `undeclared` names (typo, missing dependency): check that list.

**Live first** – change the image, then write it back:

```sh
stlive method compile Cart --file total.st     # selector and body, no Tonel wrapper
stlive test run CartTest
stlive save --package MyApp --dir src --verify
```

A new method needs `--protocol`. `save` writes nothing on `save_conflict`: a file changed after the image read it. Compare both versions; `load` takes the file's, `save --force` the image's. Never `--force` without looking.

Do not change the same method in a file and live between one load and the next save.

## Read results per command

| Command | Success | Why `ok: true` is not enough |
|---|---|---|
| `eval` | the value you expected | `value_is_exception` |
| `test run` | `result.outcome: "passed"`, the intended `targets`, `ran` > 0 | failed tests: `ok: true`, `outcome: "failed"`, exit 1; check `skipped` |
| `load` | exit 0, empty `undeclared` | `undeclared` does not fail the load |
| `save --verify`, `drift` | exit 0 | exit 1 = image and files differ; `compared` / `not_compared` say what was checked – never behaviour |
| `test run --fresh` | `outcome: "passed"` | exit 6 = the baseline did not load (`error.load_error`) |

Two timeouts: `error.kind: "timeout"` with a `session` means the image interrupted the code (nothing runs on). Exit 5 with `client_timeout: true` means the client gave up and the request may still run (`may_still_run`): check with `stlive ping` and the image state before resending anything that changes state.

## When the error is not enough: the debug session

An exception in `eval` (without `--no-session`) or a failing test keeps a debug session: `error.session`, `error.top`, `error.first_application_frame`. Usually type, message and top frame suffice; otherwise (real session ID from the output):

```sh
stlive debug frames dbg-3 --limit 5
stlive debug frame dbg-3 2
stlive debug eval dbg-3 --frame 2 'subtotal'
stlive debug rerun dbg-3        # after the fix: new run of the original operation
stlive debug terminate dbg-3
```

`debug resume` continues the old execution and proves nothing; `rerun` uses the current code in the same image, earlier side effects remain. Terminate your sessions when done.

## Done means – when you changed code

A lookup or a behaviour check that changed nothing needs none of this; report what you found and how you checked it.

1. Image and files agree: `stlive save --all --dir src --verify` after live changes, `stlive drift --all --dir src` after file edits (a `different` method there means the edit was not reloaded).
2. `git diff -- src` shows only intended changes.
3. A clean image passes: `stlive test run --fresh --load MyApp --repository "tonel://$PWD/src" --group Tests`. Without targets it runs the baseline's group `Tests` (`result.targets_from` names the rule). Check targets, counts and skips.
4. Report what changed, which checks ran with their counts, what was saved, what remains unverified. Never claim a check you did not run.

Replace `MyApp`, `Cart`, `src` and `Tests` with the project's real names.

## More

[references/workflows.md](references/workflows.md): instances, all result fields and exit codes, object inspection, load/save/drift details, tests and fresh runs, debugging, GUI, `run`/export, Pharo pitfalls, troubleshooting. Read only the section you need.
