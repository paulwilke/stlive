---
name: stlive
description: Use the `stlive` CLI for Pharo 13/14 (Smalltalk) tasks that need a running image or the project's stlive workflow – probing how code behaves, reloading edited Tonel files, running SUnit tests (also in a fresh image), examining a failure from its kept stack, and writing live changes back to Tonel. Not needed for reading or editing source files alone.
---

# stlive

Request commands print JSON: `{"ok":true,"result":…}` or `{"ok":false,"error":…}` (`run` prints the program's own output). An error's `message` and `hint` usually name the next step. Read them and change the input; never resend an unchanged failed call. Exact options: `stlive <command> --help`.

## Which image

- **The project decides.** When CLAUDE.md, AGENTS.md or the task names a reload or test script, an instance or a state directory, use those; such scripts wrap the commands below. Judge a script by its exit status and the test evidence it prints, and say so when it shows none.
- Confirm the instance before you change image state: `stlive list` shows the instances, `stlive status` the one you address. Name it on **every** call (`stlive -i NAME …`; shell variables do not survive between tool calls). `STLIVE_DIR` / `--state-dir` selects the state directory.
- Never stop, restart or reload an image someone else works in. On `not_running`, read the message: it may name the directory where that instance runs. Fix the name or path rather than starting a second image.
- Stopping your own instance: try `stlive stop`. If it refuses, `error.would_lose` counts what the image still holds. Save or snapshot what the task needs, then `stop --force`.

## Evaluate and look up

```sh
stlive eval --no-session 'Cart new total'
stlive eval --no-session <<'EOF'
(Cart new add: (Item price: 3); yourself) total = 3
EOF
stlive eval --file probe.st
stlive method show 'Cart>>total'          # class side: 'Cart class>>new'
stlive class show Cart
```

- Code with several lines or string literals goes through a quoted heredoc (`<<'EOF'`) or `--file`, never inside a single-quoted shell argument.
- Use `--no-session` for exploration, so a failed snippet leaves no suspended session.
- **Let errors signal.** Do not wrap code in `on: Error do: [ :e | e ]` to see an error: stlive already reports its type, message and top frame. An exception answered as a value comes back as `ok: true` with `result.value_is_exception: true`; unless you meant to get an exception object, your code failed.
- End a check with a comparison (`… total = 3`); an expression that merely completes proves nothing.
- Read real selectors before calling them (`class show`); an instance variable has no getter unless one is defined.
- Pipe JSON straight into `jq`, or use `printf '%s' "$out" | jq …` for captured output (`echo` can break the JSON).

## Change code: one path per method

**Files first** – edit the Tonel files, then reload:

```sh
stlive load MyApp --repository "tonel://$PWD/src" --group Tests
stlive test run 'MyApp-Tests'
```

`load` lets the files win: unsaved live changes in the reloaded packages are replaced (`modified_packages_overwritten`). If this image may hold live edits you have not saved, add `--protect-unsaved`: the load is then refused instead (`unsaved_changes`).

**Live first** – change the image, then write it back:

```sh
stlive method compile Cart --file total.st     # selector and body, no Tonel wrapper
stlive test run CartTest
stlive save --package MyApp --dir src --verify
```

`save` writes nothing on `save_conflict`: a file changed after the image read it. Compare both versions before choosing: `load` keeps the file's, `save --force` the image's.

Do not change the same method in a file and live between one load and the next save.

## What counts as success

`ok: true` says the command ran, not that the result is good.

- **eval**: the value you expected.
- **load**: `result.loaded.undeclared` and `result.loaded.warnings` are empty. Neither fails a plain load, but both fail a fresh run.
- **test run**: exit 0 and `result.outcome: "passed"`, with the `targets` you meant, `ran` > 0 and `skipped` checked. Red tests give `ok: true`, `outcome: "failed"`, exit 1.
- **test run --fresh**: the same. Without targets it picks the baseline's group `Tests`, or else added packages ending in `-Tests`; `result.targets_from` says which. Exit 6: the baseline did not load cleanly (errors, warnings or undeclared names).
- **save --verify / drift**: exit 0 means image and files agree in what `compared` lists. They never check behaviour.
- **Exit 5** (`client_timeout: true`): the client gave up waiting and the request may still run. Check with `stlive ping` before resending anything that changes state.

A failure that its message does not explain may have left a debug session: `error.session` after an eval, `result.groups[].sessions` after red tests. [Reference §7](references/workflows.md) shows how to read its frames and re-run after a fix.

## Done means – when you changed code

A lookup or behaviour check that changed nothing needs none of this: report what you found and how you checked it.

1. Follow the project's own gates when it has them.
2. Image and files agree: `stlive save --all --dir src --verify` after live changes, `stlive drift --all --dir src` after file edits.
3. `git diff -- src` shows only intended changes.
4. A clean image passes: `stlive test run --fresh --load MyApp --repository "tonel://$PWD/src" --group Tests`.
5. Report what changed, which checks ran with their counts, what was saved and what remains unverified. Never claim a check you did not run.

Replace `MyApp`, `Cart`, `src` and `Tests` with the project's real names.

## More

[references/workflows.md](references/workflows.md): instances, result fields and exit codes, object inspection, print limits, load/save/drift details, tests and fresh runs, debugging, GUI, `run`/export, Pharo pitfalls (class definitions), troubleshooting. Read only the section you need.
