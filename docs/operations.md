# Operations and troubleshooting

[Documentation home](../README.md#documentation) · [Project workflows](workflows.md) · [Command reference](commands.md)

## Storage and lifecycle

| Location | Contents | Configuration |
|---|---|---|
| Home, default `~/.stlive` | VM, template image, embedded source trees, setup logs | `STLIVE_HOME` |
| Project state, default `./.stlive` | Instance images, PID/port files, logs and screenshots | `--state-dir` / `STLIVE_DIR` |
| Your source directory, often `src` | Tonel files you review and commit | `save --dir`, `load --repository` |

If only `STLIVE_DIR` is set, it also becomes the home. Set **both** when sharing an existing template with an isolated state directory. If neither is set, the defaults apply. State directories receive a `.gitignore` so their generated files are not committed.

| Command | What it preserves or changes |
|---|---|
| `save --package P --dir src` | Writes recorded source changes to Tonel; does not snapshot runtime objects |
| `image save` | Snapshots the development image; does not write your source repository |
| `stop --force` | Ends the instance; changes since its last snapshot are lost unless separately exported as source |
| `image export PATH --force` | Saves an application image, removes the server by default and ends the current instance |

Suspended debugger sessions are not a supported way to preserve an execution across image restarts. Object/session IDs carry a run epoch and are rejected after restart. At most 5,000 object references and 60 sessions are kept; older entries may be evicted or terminated.

Use `--instance NAME` on every command for a nondefault instance. `stlive instances` lists known instances. `stlive image clone --help` describes cloning options. Coordinate before reloading, stopping or clearing sessions in an image used by someone else.

## Attach to an existing image

`attach` requires an image already exposing a compatible JSON-line `eval` endpoint, such as an older ST-Live server. It cannot discover or control an arbitrary uninstrumented Pharo process by PID, and a NeoConsole Telnet or HTTP port is not interchangeable with this protocol.

```sh
stlive init --vm /absolute/path/to/Pharo --image /absolute/path/to/Pharo.image
stlive attach --port 12345
```

Replace `12345` with the existing compatible endpoint's port. Setup uses a local image copy; attach loads the current server into the running image. The old endpoint remains running. Use this only on a development image you intend to modify.

## Output and exit codes

Most request commands print JSON containing `ok`, plus `result` or `error`. Use `--pretty` for formatted JSON or `--text` for readable output. Notable exceptions:

- `run`: program result on stdout, errors on stderr.
- `changes watch`: a stream of JSON lines rather than one final response.
- `--help` / `--version`: ordinary text.

`test run` can return `ok: true` while reporting failed tests: the test operation completed, but the CLI exits 1. Inspect `ran`, `passed`, `failed`, `skipped` and the exit code, not only `ok`.

Read errors even if an invocation completed quickly. A Pharo exception usually becomes an inspectable session; an unavailable image, invalid argument or broken connection does not necessarily create one. `eval --no-session` avoids retaining failed exploratory jobs. `--timeout` for live evaluations is in **milliseconds**; `run --timeout` is in **seconds**.

| Exit | Meaning |
|---|---|
| `0` | Command succeeded; still inspect behavioral results and test counts |
| `1` | Operation error, Pharo exception, failing tests or drift |
| `2` | Usage/setup error |
| `3` | Image unavailable or connection failure |
| `4` | Protocol error |
| `5` | Client response timeout |
| `6` | Fresh-test load failed |
| `130` / `143` | Fresh test interrupted by SIGINT / SIGTERM |

`run` has its own contract: `1` for an unhandled expression exception, `124` for timeout, or the application's exit code from `StLiveRun exit:`. See its command help.

`--log FILE` records metadata, duration, output size and a short summary; `--tag NAME` labels related calls. This is not a full command/output transcript.

## Troubleshooting

| Symptom | Next step |
|---|---|
| `not_initialized` | Check both home/state settings, then initialize the intended home. |
| No running image / unreachable | Check `status`, `instances` and the instance log before starting or stopping anything. |
| Compile succeeded, behavior unchanged | Inspect `stale_frames`; resume can still execute old methods. Reproduce again, accounting for earlier mutations. |
| Test is green but saved files fail | Save every affected package, inspect drift and the local source diff, then run fresh tests. |
| Package-cache write error after copying a template | A snapshot may retain an absolute Monticello cache path. Correct `MCCacheRepository cacheDirectory:` in your own template and snapshot it; do not change another project's image or cache. |
| `stale_ref` | Obtain a new object/session reference after the image restart. |
| `unknown_ref` / `unknown_session` | Reference may have been released or evicted; inspect current state. |
| Fresh tests ran nothing useful | Inspect targets; select test packages explicitly if they do not end in `-Tests`. |
| Background work stopped | Inspect `image info` → `background_errors` and the log. |
| `ui_unavailable` | UI process was busy or modal; resolve it before retrying. |
| Old saved image hangs at startup | Images saved by ≤0.7.x may contain a stale acceptor. Preserve a backup; rebuild that disposable instance from the template and reload saved sources. |

Do not delete the only copy of unsaved work when recovering an image. `start` reports startup log excerpts when the port file does not appear. Orphaned temporary fresh-test instances appear in `instances`; `instances --prune-temporary` removes them.

## Platform support

One binary supports **Pharo 13 and Pharo 14**. `stlive init` installs 14 by default; `stlive init --pharo 13` selects 13, and a second version is installed beside the default one (`~/.stlive/pharo13/`, `config-13.json`). `start --pharo N` and `run --pharo N` pick a version; an instance remembers the version of its image (`"pharo"` in its config, shown by `status`, `image info` and `instances`) and refuses `--pharo` of another one (`pharo_mismatch`). A template made before 0.9 has no version recorded; it is detected from the `PharoNN.x-….sources` file next to its image (or `printVersion`), never assumed. What differs: Pharo before 14 needs `st file --no-quit` to stay alive, Pharo 14 keeps running by default and rejects the flag; stlive picks the right form from the detected version. `start --gtk` (`--worker --headless`) is an unverified opt-in for Spec-Gtk style work; `--vm-arg` passes extra VM options.

| Platform | Recorded coverage | Remaining uncertainty |
|---|---|---|
| macOS arm64 | Main development platform; headless and Spec2/Morphic GUI workflows | GUI E2E last recorded for 0.7.0 |
| Linux x86-64 | Headless E2E recorded for 0.8.0; CI builds | Less exercised than macOS; no equivalent GUI claim |
| Windows 11 | Orbit verified init/start/load/eval/tests with Pharo 13 and VM 10.3.9 in 0.8.1; Pharo 14 not tried | GUI, UI commands, open-ui and E2E suite not verified |

Automatic download also depends on an upstream Pharo VM for your OS/architecture. Shell examples in these docs are not PowerShell scripts. Installation currently requires Rust; no prebuilt binaries are shipped.

## Validation and known limitations

The 0.8.0 changelog records 203 headless E2E checks, 150 server SUnit tests and 36 Rust tests; 19 GUI checks were last recorded for 0.7.0. These are historical results, not a promise that every combination is tested on every commit. The GitHub workflow builds on macOS/Linux; it does not run the Pharo E2E suite, and its Clippy command currently tolerates failures. See [CHANGELOG.md](../CHANGELOG.md) for scope.

- No single-stepping yet; resume, restart, return and rerun are available.
- Direct VM stdout/stderr writes are not captured in live evaluation results; Transcript is captured.
- Existing superclass and trait-composition changes are not rewritten by `save`; inspect structural source changes carefully.
- `eval`-compiled source can retain LF line endings; Tonel loads and `method compile` use Pharo's CR convention. Build LF explicitly for protocols that require it.
- Deep stacks are walked up to 100,000 frames; recursion summaries do not necessarily locate the initial call.
- Headless background exceptions are logged and end the failing process. GUI background errors can still open Pharo's debugger.
- No authentication or Unix-domain-socket transport. Anyone reaching the loopback port can execute code with the Pharo process's permissions.

The [agent skill](../skill/SKILL.md) provides a working method, not a model capability guarantee. A successful agent run on a small task does not establish broad Smalltalk competence, and an unsuccessful model run alone does not establish a broken CLI.

## Third-party software

ST Live is MIT licensed. Setup fetches, rather than bundles, **Pharo** ([pharo.org](https://pharo.org), MIT) and **NeoJSON** ([svenvc/NeoJSON](https://github.com/svenvc/NeoJSON), MIT). Retain their license notices when distributing an exported image containing them. `init` loads NeoJSON for applications; ST Live's JSON-line protocol itself uses STON.
