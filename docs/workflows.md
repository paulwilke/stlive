# Project workflows

[Documentation home](../README.md#documentation) · [First tutorial](getting-started.md) · [Command reference](commands.md)

These examples assume an existing Metacello baseline. Replace `MyApp`, packages and groups with the names in your repository. Shell examples use Bash-compatible syntax.

## Load an existing project

From the project root:

```sh
stlive start
stlive load MyApp --repository "tonel://$PWD/src" --group Tests
stlive test run 'MyApp-Tests'
stlive image info
```

Use the baseline name without `BaselineOf`. The group `Tests` must exist; repeat `--group` for multiple groups. Test targets can be packages, test classes, `'Class>>testSelector'` or quoted globs such as `'MyApp-*-Tests'`.

**Reloading chooses the files.** Another `load` overwrites modified loaded packages, including unsaved live edits, and refreshes the baseline package. Save first if you want to retain live changes. Read the response:

| Field | Meaning |
|---|---|
| `modified_packages_overwritten` | Live changes replaced in loaded packages |
| `stale_packages` | Packages no longer listed by the baseline, still in the image |
| `not_reloaded_packages` | Packages outside the requested groups |
| `undeclared` | Referenced names without definitions, such as a missing dependency or typo |

`load --prune` unloads stale packages, except ones with unsaved changes or listed by another loaded baseline. Plain `load` can report `undeclared` and still exit 0. Loading is not a behavioral test.

## Repair and save a method

```sh
stlive method show 'Cart>>total'
stlive method compile Cart --file total.st
stlive test run 'CartTest'
stlive changes list
stlive save --package MyApp --dir src --verify
git diff -- src
```

The file contains the selector and body. Existing methods retain their protocol unless specified; new methods need `--protocol`. For class-side compilation, quote the class name: `'Cart class'`.

Read real source and selectors first, reproduce a concrete failure, make one small change, and check the original case plus relevant edge cases. An instance variable does not imply a getter method. Use Boolean comparisons or assertions to state expected results; a successful last expression alone proves little. If a call fails, read its error and change the input or strategy before retrying. Never claim a check you did not execute.

When preserving state is part of the task, inspect and retain the supplied object reference, compile on its class and call the new method with `eval --in REF`. Do not replace the instance. See the [live-object example](getting-started.md#2-add-a-method-to-an-existing-object).

Changes through `eval` or the IDE are recorded too. Save every affected package, including tests and extensions. `save --all --dir src --verify` is useful when the directory is intended to contain all recorded changes. `drift --all --dir src` checks loaded package folders and loaded packages listed by local baselines but missing from files. Neither command runs tests.

`save` handles per-method Tonel changes, traits, extensions and removals. It does not rewrite a changed superclass or trait composition of an existing class. Inspect the diff and verify saved sources after structural changes.

## Test the saved baseline in a separate image

Save live changes first, then use the **local source path**:

```sh
stlive test run --fresh --load MyApp --repository "tonel://$PWD/src" --group Tests
```

This starts a temporary instance from the installed template, loads the baseline, runs tests, then removes the instance. The normal project instance is not replaced. A remote repository URL would test remote sources rather than uncommitted local files.

Without explicit targets, fresh mode selects packages added by the load whose names end in `-Tests`; specify targets if your naming differs. Check targets, counts, failures and skips. Use `--skips-fail` if your policy treats skips as failures. Prepared template dependencies still matter; this does not validate every deployment environment.

- Exit `1`: test failures or an operation error; read the response.
- Exit `6`: baseline load rejected, including errors, warnings or undeclared names.
- `--keep`: retain the temporary instance for debugging; its name is returned. Stop that specific instance when finished.

## Run from Tonel without a persistent server

```sh
stlive run --load src --package MyApp --eval 'MyApp run' -- arg1 arg2
```

`run` loads Tonel in a temporary image and prints the expression's value directly to stdout. Errors go to stderr. Use `--file start.st` for a longer program, and `StLiveRun arguments` for arguments after `--`.

Repeated `--package` options specify load order. `--prepare deps.st` executes **after** loading the Tonel files; dependencies needed to load class definitions must already be available. Prefer a baseline's loading procedure when checking project dependencies.

## Work on a GUI

Start a separate GUI instance, then load and open your application:

```sh
stlive --instance gui start --gui
stlive --instance gui load MyApp --repository "tonel://$PWD/src" --group Tests
stlive --instance gui eval --ui 'MyPresenter new open. 1'
stlive --instance gui ui windows
stlive --instance gui ui screenshot --window Demo
stlive --instance gui ui press Increment
```

`MyPresenter`, `Demo` and `Increment` stand for your presenter, window title and button label; this is not a standalone example. Use `--ui` for evaluations and compilation touching widgets. Captured errors in ST-Live-started UI operations become sessions; a replacement UI process keeps drawing. Errors outside these requests can still open Pharo's debugger. `ui_unavailable` means the deferred operation did not start in time.

Window enumeration and button presses support Morphic-based windows. Do not assume Bloc/Toplo support from `--ui`. For web interfaces, `stlive open-ui http://localhost:8080` opens Chrome/Edge in app mode; it does not start a web server.

## Export an application image

After saving and testing:

```sh
stlive image export dist/MyApp.image --force
```

Export saves under the new name and ends the current instance. By default it removes ST Live and the evaluation endpoint. Provide your own application startup procedure and retain [third-party notices](operations.md#third-party-software). `image save` instead preserves a development image.
