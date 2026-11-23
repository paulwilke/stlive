# stlive reference

Sections: 1 Setup and instances · 2 Output, fields and exit codes · 3 Exploring objects · 4 Reloading from files · 5 Live changes and saving · 6 Tests and fresh runs · 7 Debugging · 8 GUI · 9 `run` and export · 10 Pharo pitfalls · 11 Troubleshooting. Exact options: `stlive <command> --help`.

## 1 Setup and instances

```sh
stlive init                    # once per machine: Pharo 14 (or --pharo 13) and an image with the server, in ~/.stlive
stlive start                   # in the project directory; state in ./.stlive
stlive load MyApp --repository "tonel://$PWD/src" --group Tests
stlive image info              # version, loaded packages, unsaved changes, sessions, background_errors
```

- **Directories.** `STLIVE_HOME` holds the template (VM, image); `STLIVE_DIR` (or `--state-dir`) the instance state. With only `STLIVE_DIR` set it is both, and `~/.stlive` is never touched. To share the installed template with an isolated state directory, set both.
- **Instances.** `-i NAME` / `--instance NAME` (or `STLIVE_INSTANCE`) on every command. `stlive -i NAME start` creates the instance from the template. `stlive list` (alias of `instances`) lists the instances of the state directory, marking temporary ones from `test run --fresh` and orphaned ones; `instances --prune-temporary` removes the orphans. Separate instances with separate state directories run in parallel without interfering.
- **Versions.** `start --pharo N`, `run --pharo N`, `test run --fresh --pharo N` pick a template installed with `init --pharo N`. An instance keeps its version; `--pharo` against an image of another version fails with `pharo_mismatch`.
- **Lifetime.** Object references (`obj-…`) and sessions (`dbg-…`) die with the image process (`stale_ref` after a restart). At most 5,000 references and 60 sessions are kept.
- **Stopping.** `stop` without `--force` asks the image what it holds and stops when it reports no suspended session, no unsaved recorded change and no object reference; otherwise it refuses (`confirmation_required`, exit 2) with `error.would_lose` and the complete command. Only these three are counted: state held by the image's own globals (a running server, loaded data) is invisible to the check. An image that does not answer the question is stopped; one started by an older stlive answers without the counts and needs `--force`. `debug terminate SESSION` and `obj release --all` clear what is yours, but when you mean to discard the state anyway, `stop --force` is simpler. `stop` discards everything not snapshotted: `image save` keeps the image state, `save` keeps source. `image clone NAME` copies an instance.
- **`not_running`** (exit 3): when an instance of that name runs in another state directory that stlive knows (a `.stlive` above the current directory, or one `start` has used), the message names it; it also lists the instances running in the directory searched. No such note means none was found. Usually `STLIVE_DIR` or `-i` is wrong.

## 2 Output, fields and exit codes

Request commands print JSON (`--pretty` formatted, `--text` readable). `run` prints the program's result; `changes watch` streams JSON lines; `--help` and `--version` print text. Piping into `head` is safe.

**eval result**: `result.value` = `{class, print, ref}`; `print_truncated: true` with a `hint` when `print` was cut (default 120 characters; `--print-limit N` or `STLIVE_PRINT_LIMIT=N`). `--full` adds the whole printString as `result.text` (capped at 100,000). Transcript output and deprecation warnings arrive in `result.output.{transcript,deprecations}`, never in the value. `result.value_is_exception: true` plus a `warning`: the code answered an exception object instead of signalling it.

**Exception error**: `error.{type, message, kind: "exception"|"timeout", session, top, first_application_frame}`. `top` is the top frame: `label` (`Class>>selector`), `origin` (`application`, `framework`, `eval`, `tooling`), `statement`. Extras when they apply: `did_you_mean` (real selectors of the receiver, for `MessageNotUnderstood`), `hint` (for example the class-definition syntax), `recursion.cycle` (repeating frames of an endless recursion). With `--no-session`: `session_ended: true`, no session kept. `--no-source` leaves out `statement`.

**Compile error** (`method compile`, `eval`): `error.code` `compile_error` (undeclared name, with `variable`) or `syntax_error`, with `line` and `column`.

**Timeouts**

| What you see | Meaning | Still running? |
|---|---|---|
| `error.kind: "timeout"`, `error.session`, exit 1 | The image interrupted code that ran past `--timeout` (milliseconds) and kept it as a session | No (`may_still_run: false`) |
| `error.code: "timeout"`, `client_timeout: true`, exit 5 | The client stopped waiting | Possibly (`may_still_run: true`); `stlive ping`, then check state |

**Exit codes**: 0 ok (still read the counts) · 1 operation error, exception, failing tests, drift, `save_conflict` · 2 usage, setup, `confirmation_required` · 3 image unavailable · 4 protocol error · 5 client timeout · 6 `test run --fresh`: the load failed · 130/143 fresh run interrupted. `run` has its own codes (section 9).

**Shell**: pipe output directly into `jq` (see section 11 for stored output). `--log FILE --tag NAME` appends one summary line per call.

## 3 Exploring objects

- Code: `method show 'Class>>sel'`, `class show Foo` (or `'Foo class'`), `find implementors|senders SELECTOR`, `find class GLOB`, `find package GLOB`, `package list GLOB`, `test list GLOB`.
- Objects: a non-literal value comes with a `ref`. `obj show REF` (instance variables, paged with `--offset`/`--limit`), `obj items REF` (collection elements), `obj var REF NAME`, `obj text REF` (full text, paged by characters), `obj compare A B`, `obj graph REF --depth 3` (identity, shared nodes, cycles), `obj referrers REF`, `obj release REF|--all`.
- `eval --in REF '…'` runs code with that object as `self`. When a task hands you a live object whose state or identity must survive, keep its reference, compile the method on its class and call it with `eval --in REF`; do not replace the instance.
- `image info` lists loaded packages and unsaved changes; `changes list` / `changes show N` the recorded changes (`unsaved_by_package`).

## 4 Reloading from files

`stlive load BASELINE --repository tonel://ABS/src [--group G]…` (baseline name without `BaselineOf`). Re-running it after editing files updates the loaded packages. Read the answer:

| Field | Meaning |
|---|---|
| `modified_packages_overwritten` | Packages whose live changes the files replaced; their unsaved changes are dropped as superseded |
| `baseline_package` | `reloaded` when the baseline itself changed |
| `stale_packages` | Loaded packages the baseline no longer lists; `load --prune` unloads them (`prune_skipped` keeps ones with unsaved changes or listed by another baseline) |
| `not_reloaded_packages` | Loaded packages outside the requested groups; load again with that `--group` |
| `undeclared` | Names the code uses and nothing defines; the load still exits 0 |

`--protect-unsaved` refuses the load (`unsaved_changes`, nothing loaded) while the baseline's packages have unsaved changes – use it whenever the image may hold live edits you have not saved. Without it the files win. Loading is not a test.

`drift --all --dir src` after editing files shows whether the image matches them: a method listed in `different` or `missing_in_image` differs between image and file. After a file edit that usually means it has not been reloaded; the report does not say which side changed.

## 5 Live changes and saving

**Compile.** `method compile CLASS --file m.st` (also stdin, `--source`). The file holds selector and body, not a Tonel wrapper. An existing method keeps its protocol; give a new one `--protocol NAME` (`'*Pkg'` for an extension of package Pkg), otherwise it is filed under `as yet unclassified` and the result carries a `note`. Class side: `method compile 'Cart class' …`. The result lists `stale_frames`: suspended executions still running the old code. `method remove` is destructive.

**Classes and traits.** `class create Foo --superclass Bar --ivars 'a b' --package Pkg` (also `--class-vars`, `--class-ivars`, `--traits 'TFoo + TBar'`, `--comment`); `class create TFoo --trait --package Pkg`. Compile trait methods into the trait (`'TFoo classSide'` for its class side), not into a user of the trait; `save` writes them to `TFoo.trait.st`. Class comments: change them in the image (`--comment` or `Foo comment: '…'`), `save` writes them.

Changes made through `eval` (`compile:`, new variables) or the IDE are recorded and saved too.

**Save.** `save --package P --dir src` or `save --all --dir src`; `--dry-run` shows the plan; `--verify` compares afterwards and fails on any remaining difference. `save` plans all packages before writing the first file.

- **Conflict check.** Per method, `save` compares the source the image started from, its current source and the file. A file that is neither was changed after the image read it (an editor, a branch switch, a pull): `save_conflict`, exit 1, `error.conflicts` lists method, file and problem, and nothing in any package is written. Compare both versions (`method show`, the file); `load` takes the files', `save --force` the image's (`conflicts_overwritten`). Whitespace alone is no conflict. Class definitions are not conflict-checked.
- **Definitions.** A changed superclass or trait composition of an existing class is written; a slot definition (`'x => SomeSlot'`) is kept when the variable names did not change.
- **Verify / drift.** Each package report has `missing_in_files`, `missing_in_image`, `different`, `classes_missing_in_files`, `classes_missing_in_image`, `definitions_different`, plus `compared` and `not_compared`. Not compared: class comments, method protocols, slot and class-variable definitions beyond their names, and behaviour. `drift --all` also reports baseline packages loaded without a folder (`packages_missing_in_files`). An image started by an older stlive sends no definitions; `compared` then lists only methods and names.

Then read `git diff -- src`: it should be minimal.

## 6 Tests and fresh runs

`test run TARGET…` takes packages, classes, `'Class>>testSel'` and quoted globs (`'App-*-Tests'`); several targets form one run. A literal target that matches nothing fails with `no_tests` and suggests similar classes; a glob that matches nothing fails with `no_tests_matched`. It runs what SUnit runs: inherited tests of an abstract superclass, the concrete subclasses of an abstract class, each parameter combination of a `ParametrizedTestCase` (`Class>>sel (#param->value)`, also a valid target).

Result: `outcome` (`passed`/`failed`, as the exit code counts it), `targets`, `ran`, `passed`, `failed`, `skipped`, `skipped_tests`, `duration_ms`, `groups[]` – failures grouped by cause with `type`, `message`, `first_application_frame`, `top_frames`, `tests` and, by default, one kept session per group. `ok: true` only says the run took place. Skipped tests are neither passed nor failed; `--skips-fail` makes them fail the run. Large red runs: one session per group is kept, the kept session IDs are in `groups[].sessions`, and `groups[].sessions_ended` counts the ones that were ended; `--keep-sessions` keeps all (small runs only), `--discard-sessions` none. `--timeout` is milliseconds per test.

**Fresh run** – does the saved source load and pass in a new image?

```sh
stlive test run --fresh --load MyApp --repository "tonel://$PWD/src" --group Tests
```

It starts a temporary instance from the template, loads, runs, and removes it; your instance is untouched. Use the local `tonel://` path, otherwise you test remote sources. Without targets it runs the test packages of the baseline's group `Tests` (those defining a `TestCase` subclass); a baseline without that group, or a load without its test packages, falls back to added packages ending in `-Tests`. `result.targets_from` names the rule, `result.load.tests_group` the group; `no_tests_matched` says when to add `--group Tests`. Exit 6 (`load_failed`): the load raised an error, reported warnings or left undeclared names – see `error.message`, `result.load.undeclared` and, when the load itself raised an error, `error.load_error`. `--keep` keeps the instance for `stlive -i NAME debug …` (stop it afterwards); `--load-timeout` (ms), `--pharo N`. The template may hold prepared dependencies, so this is not every deployment environment.

## 7 Debugging

A kept session survives until terminated, the image stops, or the 60-session limit evicts it.

- `debug list`, `debug inspect S` – sessions and a compact description.
- `debug frames S --limit 5 [--offset N]` – a page of frames with `origin`; start at `first_application_frame`, never print the whole stack.
- `debug frame S N [--no-source]` – executing statement, receiver, variables with `kind` (argument, temporary, block_argument, block_temporary, outer_block_*). `debug locals S`, `debug receiver S --frame N` (an object reference).
- `debug eval S --frame N 'expr'` (also `--file`) – evaluate with that frame's variables visible. Experiment rather than guess.
- After a fix: `debug rerun S` runs the original operation again with the current code in the same image; earlier side effects remain, and `fresh_execution` does not mean a fresh image. `debug restart S --frame N` restarts one frame (only when that is semantically right). `debug resume` continues the OLD execution, possibly in old methods (`stale_frames`), and proves nothing. `debug return` makes a frame answer a value.
- `debug terminate S` (or `--all`, only for sessions that are yours).
- Endless loops: `eval --timeout MS` turns them into a session. Endless recursion: `error.recursion.cycle`.
- Errors in processes your code forked do not end a headless image: they are logged and listed under `image info` → `background_errors` (type, message, frames). Check there when background work stops.
- There is no step into / step over.

## 8 GUI (Spec2/Morphic)

```sh
stlive -i gui start --gui
stlive -i gui eval --ui 'MyPresenter new open. 1'
stlive -i gui ui windows
stlive -i gui ui screenshot --window 'Demo'     # writes a PNG: look at it
stlive -i gui ui press Increment
```

Anything touching widgets runs with `--ui` (`eval`, `method compile`, `debug eval`) in the UI process; without it code runs in a worker process. Errors in stlive-started UI work become ordinary sessions; errors from a person clicking still open Pharo's debugger. `ui_unavailable`: the UI process was busy or a modal dialog is open; nothing ran. Window listing and button presses cover Morphic-based windows, not Bloc/Toplo. For a web UI: `stlive open-ui http://localhost:8080` (it starts no server).

## 9 `run` and export

`stlive run --load src [--package P]… [--prepare deps.st] (--eval EXPR | --file start.st) -- args…` loads Tonel into a temporary image and prints the expression's value to stdout (strings raw), errors to stderr. `--package` fixes the load order; `--prepare` runs **after** loading. In the program: `StLiveRun arguments`, `StLiveRun exit: n`, `StLiveRun stdout:` / `stderr:` (use these for non-ASCII output, not `Stdio stdout`). Exit codes: 0, 1 unhandled exception, your `exit:` code, 124 timeout (`--timeout` in seconds, default 300), 2 usage. Put the Smalltalk in a file so shell quoting cannot mangle it.

`stlive image export dist/MyApp.image --force` saves a copy without the stlive server and ends the instance. `image save` snapshots the development image instead.

## 10 Pharo pitfalls

- **Class definitions**: `subclass:instanceVariableNames:classVariableNames:package:` and its relatives do not exist on Pharo 13/14; the MNU carries a `hint`. Use `Object << #Foo slots: { #a }; package: 'Pkg'; install` or `stlive class create`.
- **Line endings**: methods loaded from Tonel or compiled with `method compile` get CR in multi-line string literals. Build LF explicitly where a protocol needs it: `String lf join: #(…)` or `aString withUnixLineEndings`.
- **JSON**: NeoJSON is loaded by `init` (`NeoJSONWriter toString:`, `NeoJSONReader fromString:`). `STONJSON` cannot write an `OrderedCollection`. STON's `shared reference detected` comes from one literal (`#()`, `''`) used in several fields; use fresh objects.
- **Semaphores**: `waitTimeoutSeconds:` / `waitTimeoutMilliseconds:` answer `true` when the wait timed out.
- `eval` and `debug eval` timeouts are milliseconds; `run --timeout` is seconds.

## 11 Troubleshooting

| Symptom | Next step |
|---|---|
| `not_running`, but the image is up | The message names where it runs; fix `STLIVE_DIR`, `--state-dir` or `-i` |
| `not_initialized` | Check `STLIVE_HOME` / `STLIVE_DIR`, then `stlive init` for the intended home |
| `confirmation_required` on `stop` | Read `would_lose`; terminate your sessions, save changes, `obj release --all` – or `--force` in your own instance |
| `save_conflict` | Nothing written; compare, then `load` (files win) or `save --force` (image wins) |
| `unsaved_changes` on `load` | `--protect-unsaved` refused; `save` first, or load without the option to drop them |
| Compiled, behaviour unchanged | Look at `stale_frames`; reproduce again instead of resuming |
| Green in the image, red in a fresh run | Save every affected package (tests and extensions too), `drift --all`, then the fresh run again |
| Fresh run ran nothing useful | Check `targets` and `targets_from`; add `--group Tests` or name targets |
| `stale_ref`, `unknown_ref`, `unknown_session` | The image restarted or the reference was released or evicted; get a new one |
| Background work stopped | `image info` → `background_errors`, and the instance log |
| `ui_unavailable` | Close the modal dialog or wait; retry |
| Instance never comes up (no port file, VM at 100 % CPU) | Image saved by stlive 0.7.x or older: read the log lines `start` prints, delete `.stlive/NAME.image` (keep a backup if it holds unsaved work), `start` again and reload |
| Exit 5 | The client gave up; `stlive ping`, inspect state, then decide whether to resend |
| jq fails on output stored in a variable | `echo` under `/bin/sh` turns `\n` inside JSON strings into newlines; pipe directly or use `printf '%s' "$out" \| jq …` |
| A 0.10 field or option is missing (`value_is_exception`, `--print-limit`, `unsaved_changes` in `status`, definitions in `drift`; `stop` always wants `--force`) | The instance still runs the server of the stlive that started it; the server is upgraded at the next `start`. Client-side checks (`outcome`, `client_timeout`, the `save` conflict check) work regardless. Do not restart a shared image for this – tell its owner |
