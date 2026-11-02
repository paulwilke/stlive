---
name: stlive
description: Work inside a running Pharo (Smalltalk) image with the `stlive` CLI – evaluate code, inspect live objects, debug failing code frame by frame, fix methods in the running image, re-run, run tests and write the fix back to Tonel files. Use for Pharo/Smalltalk tasks where the behaviour of running code matters (failing tests, wrong results, unknown library internals).
---

# ST Live: compact working loop

Use the existing project instance and baseline. Do not restart a shared image or reload files over unsaved live changes without coordinating with its owner. Each command returns JSON with `ok`; read the actual result before choosing the next action.

## Start with these operations

```sh
stlive method show 'Cart>>total'
stlive class show Cart
stlive find implementors lookupCode:ifAbsent:
stlive eval - < reproduce.st
stlive debug frames <session> --limit 5
stlive debug frame <session> <index>
stlive debug eval <session> --frame <index> 'subtotal'
stlive method compile Cart --file total.st
stlive debug rerun <session>
stlive test run 'CartTest'
stlive save --package MyApp --dir src --verify
```

Use real names from the project and real session IDs returned by the tool. A method file contains the selector and body, not a Tonel class wrapper. Preserve the existing protocol; a new method needs `--protocol`. For code containing Smalltalk strings, prefer `--file` or stdin over nested shell quotes.

`Cart>>total` is an instance method: call it on an instance such as `Cart new`, using the project's actual initialization. `Cart class>>...` is class-side. Read existing source and selectors before inventing APIs. Follow `did_you_mean` suggestions only after checking their meaning.

## Repair and inspect

Reproduce the reported behavior or run the relevant test. An exception can return `error.session` and `first_application_frame`; inspect a small page of frames, then the relevant frame's source and variables. Follow object references only as needed. If the task supplies a live object, inspect it with `obj show`, keep its reference, compile on its class and use `eval --in REF` to call the new method on that same instance. Do not replace it when state or identity must survive. An instance variable does not automatically provide a getter.

Compile the smallest justified change. Check the original failure plus relevant edge cases, preferably with explicit Boolean comparisons or assertions. After an error, read its message and stack; change the input or strategy before retrying rather than repeating an unchanged failed call.

- `debug resume` continues the old execution; existing frames may still use old methods.
- `debug restart` restarts one frame; it does not reset the image or object state.
- `debug rerun` executes the original operation again in the **same image**. `fresh_execution: true` does not mean a fresh image, rollback, or verified repair. Earlier side effects remain.

For a small, stateless source-editing task, ordinary Pharo scripts and tests in a separate fresh image are also valid. Use live sessions when preserved runtime state helps; ST Live is not required for every Pharo operation.

## Finish with evidence

Check behavior against the requested contract, including relevant edge cases. Use actual fixture values; an expression completing successfully does not prove its answer is correct. When adding a regression test, establish that it detects the original bug where practical.

Save the changed packages, inspect the Git diff, and check source agreement with `save --verify` or `drift`. These commands **do not run acceptance tests**. For a project with a local Metacello baseline, check the saved files separately:

```sh
stlive test run --fresh --load MyApp --repository "tonel://$PWD/src" --group Tests
```

Replace `MyApp` and `Tests` with the project's actual baseline and group. This uses a temporary image from the installed template, which may contain prepared dependencies; it does not validate arbitrary deployment environments. Check the returned targets, counts and skipped tests. `test run` may return `ok: true` with failed tests and exit 1; inspect counts and exit status. A zero-test run is not evidence that the requested behavior passed. If a fresh check is unavailable, say so.

Report what changed, which checks actually ran and their results, whether files were saved, and what remains unverified. Do not infer success from a command name or claim checks you did not run. Keep unresolved debugger sessions available if needed; clean up only your own task's sessions.

## More detail when needed

Read [workflows and troubleshooting](references/workflows.md) for setup, baseline reload behavior, paged objects, traits, GUI/UI-process work, export, exit codes and troubleshooting. Use `stlive <command> --help` for exact arguments. Load only the relevant section rather than the entire reference for a small task.
