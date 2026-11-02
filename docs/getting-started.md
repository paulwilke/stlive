# Getting started

[Documentation home](../README.md#documentation) · [Command reference](commands.md)

These examples create their own classes in a disposable image. You need an installed `stlive`, a completed `stlive init`, `jq`, and a Bash-compatible shell. Use a new directory, not an existing application's instance. Example output below is shortened; reference IDs vary between runs.

## 1. Repair a failing method

### Create a counter

```sh
mkdir stlive-demo
cd stlive-demo
stlive start
stlive class create StLiveDemoCounter --ivars value --package StLiveDemo
stlive method compile StLiveDemoCounter --protocol counting - <<'ST'
next
    ^ value := value + 1
ST
stlive eval 'Smalltalk globals at: #StLiveDemoObject put: StLiveDemoCounter new. 1'
```

The counter's `value` starts as `nil`. `StLiveDemoObject` holds the same instance between commands. A method input contains its selector and body, not a Tonel class wrapper.

### Reproduce and inspect

The next evaluation deliberately exits **1**. Run interactively, or handle that expected failure in a script with `set -e`.

```sh
stlive eval 'StLiveDemoObject next' > failure.json
session=$(jq -r '.error.session' failure.json)
cat failure.json
stlive debug frames "$session" --limit 5
frame=$(jq -r '.error.first_application_frame' failure.json)
stlive debug frame "$session" "$frame"
stlive debug eval "$session" --frame "$frame" 'value'
```

Selected fields from an actual Pharo 13 run (your IDs will differ):

```json
{
  "ok": false,
  "error": {
    "type": "MessageNotUnderstood",
    "message": "receiver of \"+\" is nil",
    "session": "dbg-2c7-1",
    "first_application_frame": 0
  }
}
```

The failed process remains suspended after the CLI exits. Use the returned application frame instead of assuming a stack depth. Its source is `StLiveDemoCounter>>next`, and the inspected `value` is `nil`. Debug evaluation can change state; this expression only reads it.

### Add a test and fix the method

```sh
stlive class create StLiveDemoCounterTest --superclass TestCase --package StLiveDemo-Tests
stlive method compile StLiveDemoCounterTest --protocol tests - <<'ST'
testFirstValue
    self assert: StLiveDemoCounter new next equals: 1
ST
stlive test run StLiveDemoCounterTest
```

This deliberately fails (exit **1**) against the old method. Now repair it:

```sh
stlive method compile StLiveDemoCounter --protocol counting - <<'ST'
next
    ^ value := (value ifNil: [ 0 ]) + 1
ST
stlive debug rerun "$session"
stlive test run StLiveDemoCounterTest
```

The rerun returns `1`; the test reports one passing test. It checks a newly created counter independently of the suspended object. Rerun uses the **same image and object**; calling `StLiveDemoObject next` again advances to `2`, rather than resetting the counter.

## 2. Add a method to an existing object

Continue in the same tutorial directory and image. Keep the instance's reference rather than creating a replacement:

```sh
stlive eval 'StLiveDemoObject' > object.json
ref=$(jq -r '.result.value.ref' object.json)
stlive obj show "$ref"
stlive class show StLiveDemoCounter
```

The instance variable `value` does not imply that a getter named `value` exists. Add just the requested method, using the variable directly:

```sh
stlive method compile StLiveDemoCounter --protocol accessing - <<'ST'
currentValue
    ^ value
ST
stlive eval --in "$ref" 'self currentValue = 1'
stlive eval --in "$ref" 'self next = 2'
stlive eval --in "$ref" 'self == (Smalltalk globals at: #StLiveDemoObject)'
stlive obj show "$ref"
```

Selected fields from the two actual `obj show` responses:

```json
{
  "before": {
    "ref": "obj-2c7-2",
    "identity": 766355456,
    "instance_variables": [
      {
        "class": "SmallInteger",
        "print": "1",
        "name": "value"
      }
    ]
  },
  "after": {
    "ref": "obj-2c7-2",
    "identity": 766355456,
    "instance_variables": [
      {
        "class": "SmallInteger",
        "print": "2",
        "name": "value"
      }
    ]
  }
}
```

All three evaluations return `true`: the new method sees the old state, incrementing still works, and the receiver is still the global object's exact instance. The same object reference remains valid. This demonstrates live method addition without replacing the instance; it does not require a debugger session. References expire when the image restarts.

## 3. Save and check the source files

```sh
stlive save --package StLiveDemo --dir src --verify
stlive save --package StLiveDemo-Tests --dir src --verify
stlive run --load src --package StLiveDemo --package StLiveDemo-Tests \
  --eval 'StLiveDemoCounterTest suite run hasPassed'
```

The first two commands export classes and methods to `src/StLiveDemo` and `src/StLiveDemo-Tests`. The last loads these files in a temporary image and prints **`true`**, without a JSON envelope. The runtime global `StLiveDemoObject` and suspended executions are not part of the Tonel source.

For a real project with a Metacello baseline, use the structured [fresh-test workflow](workflows.md#test-the-saved-baseline-in-a-separate-image), checking test counts and skips. If you use Git, inspect the source diff before committing.

## 4. End the disposable instance

```sh
stlive stop --force
```

This discards the tutorial's objects and sessions; the Tonel files remain. Use an image snapshot when you want to preserve development state; see [storage and lifecycle](operations.md#storage-and-lifecycle).
