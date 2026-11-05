//! `stlive init`: download Pharo 13 (VM + image) and build an image containing the StLive server.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Stage 1 of `init` (needs network, runs once and is cached as neojson.image): JSON for applications. NeoJSON is MIT licensed (Sven Van Caekenberghe) and fetched from upstream, never bundled.
const NEOJSON_ST: &str = r#"[ Metacello new
	baseline: 'NeoJSON';
	repository: 'github://svenvc/NeoJSON:v18/repository';
	load: #( 'core' ) ]
	on: Error
	do: [ :e | Stdio stdout nextPutAll: 'stlive: NeoJSON not loaded: ' , e messageText asString; lf; flush ].
Smalltalk snapshot: true andQuit: true.
"#;

const PREPARE_ST: &str = r#"| env src packagesFile |
env := OSEnvironment current.
src := (env at: 'STLIVE_SRC') asFileReference.
Metacello new
	baseline: 'StLive';
	repository: 'tonel://' , src fullName;
	load.
(Smalltalk globals at: #StLiveServer) recordSystemPackages.
(Smalltalk globals at: #StLiveServer) installedHash: (OSEnvironment current at: 'STLIVE_SOURCES_HASH' ifAbsent: [ nil ]).
packagesFile := (env at: 'STLIVE_PACKAGES_FILE' ifAbsent: [ (src parent / 'system-packages.txt') fullName ]) asFileReference.
packagesFile ensureDelete.
packagesFile writeStreamDo: [ :out |
	PackageOrganizer default packages do: [ :pk | out nextPutAll: pk name; lf ] ].
Smalltalk snapshot: true andQuit: true.
"#;

/// (relative path, content) of the Smalltalk sources compiled into the binary.
const SOURCES: &[(&str, &str)] = &[
    ("BaselineOfStLive/package.st", include_str!("../smalltalk/src/BaselineOfStLive/package.st")),
    ("BaselineOfStLive/BaselineOfStLive.class.st", include_str!("../smalltalk/src/BaselineOfStLive/BaselineOfStLive.class.st")),
    ("StLive/package.st", include_str!("../smalltalk/src/StLive/package.st")),
    ("StLive/StLiveBackgroundErrorHandler.class.st", include_str!("../smalltalk/src/StLive/StLiveBackgroundErrorHandler.class.st")),
    ("StLive/StLiveCommands.class.st", include_str!("../smalltalk/src/StLive/StLiveCommands.class.st")),
    ("StLive/StLiveDescriber.class.st", include_str!("../smalltalk/src/StLive/StLiveDescriber.class.st")),
    ("StLive/StLiveErrorHandler.class.st", include_str!("../smalltalk/src/StLive/StLiveErrorHandler.class.st")),
    ("StLive/StLiveFailure.class.st", include_str!("../smalltalk/src/StLive/StLiveFailure.class.st")),
    ("StLive/StLiveJob.class.st", include_str!("../smalltalk/src/StLive/StLiveJob.class.st")),
    ("StLive/StLiveServer.class.st", include_str!("../smalltalk/src/StLive/StLiveServer.class.st")),
    ("StLive/StLiveSession.class.st", include_str!("../smalltalk/src/StLive/StLiveSession.class.st")),
    ("StLive/StLiveRun.class.st", include_str!("../smalltalk/src/StLive/StLiveRun.class.st")),
    ("StLive/StLiveTranscriptTee.class.st", include_str!("../smalltalk/src/StLive/StLiveTranscriptTee.class.st")),
    ("StLive/StLiveTranscriptProxy.class.st", include_str!("../smalltalk/src/StLive/StLiveTranscriptProxy.class.st")),
    // the StLive-Tests package: loaded only by the Tests group (`stlive load StLive --group Tests`), never by init
    ("StLive-Tests/package.st", include_str!("../smalltalk/src/StLive-Tests/package.st")),
    ("StLive-Tests/StLiveTestCase.class.st", include_str!("../smalltalk/src/StLive-Tests/StLiveTestCase.class.st")),
    ("StLive-Tests/StLiveBaselinePackagesTest.class.st", include_str!("../smalltalk/src/StLive-Tests/StLiveBaselinePackagesTest.class.st")),
    ("StLive-Tests/StLiveBaselineShapeTest.class.st", include_str!("../smalltalk/src/StLive-Tests/StLiveBaselineShapeTest.class.st")),
    ("StLive-Tests/StLiveBaselineVersionTest.class.st", include_str!("../smalltalk/src/StLive-Tests/StLiveBaselineVersionTest.class.st")),
    ("StLive-Tests/StLiveChangeLogTest.class.st", include_str!("../smalltalk/src/StLive-Tests/StLiveChangeLogTest.class.st")),
    ("StLive-Tests/StLiveClassLookupTest.class.st", include_str!("../smalltalk/src/StLive-Tests/StLiveClassLookupTest.class.st")),
    ("StLive-Tests/StLiveCommandsHelpersTest.class.st", include_str!("../smalltalk/src/StLive-Tests/StLiveCommandsHelpersTest.class.st")),
    ("StLive-Tests/StLiveCommandsLineEndingsTest.class.st", include_str!("../smalltalk/src/StLive-Tests/StLiveCommandsLineEndingsTest.class.st")),
    ("StLive-Tests/StLiveCommentRecordingTest.class.st", include_str!("../smalltalk/src/StLive-Tests/StLiveCommentRecordingTest.class.st")),
    ("StLive-Tests/StLiveCompileErrorTest.class.st", include_str!("../smalltalk/src/StLive-Tests/StLiveCompileErrorTest.class.st")),
    ("StLive-Tests/StLiveDescriberTest.class.st", include_str!("../smalltalk/src/StLive-Tests/StLiveDescriberTest.class.st")),
    ("StLive-Tests/StLiveServerTest.class.st", include_str!("../smalltalk/src/StLive-Tests/StLiveServerTest.class.st")),
    ("StLive-Tests/StLiveSourceNormalisationTest.class.st", include_str!("../smalltalk/src/StLive-Tests/StLiveSourceNormalisationTest.class.st")),
    ("StLive-Tests/StLiveSuggestionsTest.class.st", include_str!("../smalltalk/src/StLive-Tests/StLiveSuggestionsTest.class.st")),
    ("StLive-Tests/StLiveUndeclaredReferencesTest.class.st", include_str!("../smalltalk/src/StLive-Tests/StLiveUndeclaredReferencesTest.class.st")),
];

/// Startup script run by every `start`: upgrades the server inside the image if it was built from other sources, then starts it.
/// The sources come from this binary's own directory, so the image may have registered the StLive baseline from another
/// one (the template, or an earlier binary): `onConflictUseIncoming` replaces that registration instead of refusing.
const UPGRADE_ST: &str = r#"| env hash current run code |
env := OSEnvironment current.
hash := env at: 'STLIVE_SOURCES_HASH' ifAbsent: [ nil ].
current := (Smalltalk globals at: #StLiveServer ifAbsent: [ nil ]) ifNotNil: [ :c | (c respondsTo: #installedHash) ifTrue: [ c installedHash ] ].
(hash notNil and: [ hash ~= current and: [ (env at: 'STLIVE_SRC' ifAbsent: [ nil ]) notNil ] ]) ifTrue: [
	[ Metacello new baseline: 'StLive'; repository: 'tonel://' , (env at: 'STLIVE_SRC'); onConflictUseIncoming; load.
	(Smalltalk globals at: #StLiveServer) installedHash: hash ]
		on: Error
		do: [ :e | Stdio stdout nextPutAll: 'stlive: could not upgrade the server in this image: ' , e messageText asString; lf; flush ] ].
"#;

/// Startup script run by every `start`: upgrade (if needed), then start the server. Its lines (stdout) go to the
/// instance log: `server starting`, then `port file written` (from startFromEnvironment) or `server did not start`.
pub fn serve_st() -> String {
    format!("{}{}", UPGRADE_ST, SERVE_BODY_ST)
}

const SERVE_BODY_ST: &str = r#"Stdio stdout nextPutAll: 'stlive: server starting'; lf; flush.
[ (Smalltalk globals at: #StLiveServer) startFromEnvironment ]
	on: Error
	do: [ :e |
		Stdio stdout nextPutAll: 'stlive: server did not start: ' , e class name , ': ' , e messageText asString; lf; flush.
		Smalltalk exitFailure ].
"#;

/// Script of `stlive run`: upgrade (if needed), load sources, evaluate, write results to files.
pub fn run_st() -> String {
    format!("{}{}", UPGRADE_ST, RUN_BODY_ST)
}

const RUN_BODY_ST: &str = r#"run := Smalltalk globals at: #StLiveRun.
run beginIn: (env at: 'STLIVE_RUN_DIR').
[ [
	| packages |
	packages := (STON fromString: (env at: 'STLIVE_RUN_PACKAGES' ifAbsent: [ '[]' ])) asArray.
	(STON fromString: (env at: 'STLIVE_RUN_LOAD' ifAbsent: [ '[]' ])) do: [ :dir | run loadTonelFrom: dir packages: packages ].
	(env at: 'STLIVE_RUN_PREPARE' ifAbsent: [ nil ]) ifNotNil: [ :file | Smalltalk compiler evaluate: file asFileReference contents ].
	code := (env at: 'STLIVE_RUN_FILE' ifAbsent: [ nil ])
		ifNotNil: [ :file | file asFileReference contents ]
		ifNil: [ env at: 'STLIVE_RUN_EXPR' ].
	run result: (Smalltalk compiler evaluate: code) ]
	on: Error
	do: [ :e | run failed: e ] ] ensure: [ run finish ].
Smalltalk exit: 0.
"#;

/// FNV-1a hash (hex) of everything that ends up inside the image: the Smalltalk sources and the scripts.
pub fn sources_hash() -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    let mut feed = |s: &str| {
        for b in s.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    };
    for (name, content) in SOURCES {
        feed(name);
        feed(content);
    }
    feed(PREPARE_ST);
    feed(NEOJSON_ST);
    feed(&serve_st());
    feed(&run_st());
    format!("{:016x}", h)
}

/// Where the template (VM, image, sources) lives: `STLIVE_HOME`; without it `STLIVE_DIR` (an environment that
/// sets only the state directory must never read or write `~/.stlive`); otherwise `~/.stlive`.
pub fn home_dir() -> PathBuf {
    home_dir_from(std::env::var("STLIVE_HOME").ok().as_deref(), std::env::var("STLIVE_DIR").ok().as_deref())
}

fn home_dir_from(stlive_home: Option<&str>, stlive_dir: Option<&str>) -> PathBuf {
    if let Some(h) = stlive_home.filter(|s| !s.trim().is_empty()) {
        return PathBuf::from(h);
    }
    if let Some(d) = stlive_dir.filter(|s| !s.trim().is_empty()) {
        return PathBuf::from(d);
    }
    crate::platform::home().join(".stlive")
}

fn find_vm(pharo_dir: &Path) -> Option<PathBuf> {
    crate::platform::vm_candidates()
        .iter()
        .map(|p| pharo_dir.join(p))
        .find(|p| p.exists())
}

fn run(cmd: &mut Command, what: &str) -> Result<(), String> {
    let status = cmd.status().map_err(|e| format!("{}: {}", what, e))?;
    if status.success() { Ok(()) } else { Err(format!("{} failed ({})", what, status)) }
}

/// Write the embedded Smalltalk sources below `dir`, one file per entry of SOURCES.
fn write_source_tree(dir: &Path) -> Result<(), String> {
    for (rel, content) in SOURCES {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::write(&path, content).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Where this binary's sources are: `<home>/src-<sources hash>`. Every stlive version has its own directory, so two
/// binaries sharing one home never overwrite each other's sources (start, run and attach load from here).
pub fn sources_dir(home: &Path) -> PathBuf {
    home.join(format!("src-{}", sources_hash()))
}

/// Make sure `sources_dir(home)` holds this binary's sources and answer it. The tree is filled under a temporary name
/// and renamed into place, so a concurrent start never loads a half-written tree; an existing tree is never rewritten.
pub fn write_sources(home: &Path) -> Result<PathBuf, String> {
    let dir = sources_dir(home);
    if dir.is_dir() {
        return Ok(dir);
    }
    let name = dir.file_name().unwrap_or_default().to_string_lossy().into_owned();
    let tmp = home.join(format!("{}.tmp-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    if let Err(e) = write_source_tree(&tmp) {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(e);
    }
    match std::fs::rename(&tmp, &dir) {
        Ok(()) => Ok(dir),
        // another process put its complete tree there first (same name, same content)
        Err(_) if dir.is_dir() => {
            let _ = std::fs::remove_dir_all(&tmp);
            Ok(dir)
        }
        Err(e) => Err(format!("{}: {}", dir.display(), e)),
    }
}

/// Names of the packages of a pristine image (written by `init`), for `attach`.
pub fn system_packages(home: &Path, version: Option<u32>) -> Option<Vec<String>> {
    let file = match (version, default_template(home)) {
        (Some(n), Some(d)) if d.version != n => home.join(format!("system-packages-{}.txt", n)),
        _ => home.join("system-packages.txt"),
    };
    let t = std::fs::read_to_string(file).ok()?;
    Some(t.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
}

/// `std::fs::copy`, but a no-op when source and destination are the same file (copying a file onto itself truncates it to 0 bytes).
fn copy_unless_same(src: &Path, dst: &Path) -> std::io::Result<()> {
    if let (Ok(a), Ok(b)) = (std::fs::canonicalize(src), std::fs::canonicalize(dst)) {
        if a == b {
            return Ok(());
        }
    }
    std::fs::copy(src, dst).map(|_| ())
}

/// Pharo major version used when `init` has nothing to go by (fresh home, no `--pharo`).
pub const DEFAULT_PHARO: u32 = 14;

/// The Pharo versions stlive knows how to download and run.
pub const SUPPORTED_PHARO: &[u32] = &[13, 14];

/// Major version out of a `printVersion` line or a `PharoNN.x-….sources` file name: the digits after "Pharo".
fn major_after_pharo(text: &str) -> Option<u32> {
    let i = text.find("Pharo")? + 5;
    let digits: String = text[i..].chars().take_while(|c| c.is_ascii_digit()).collect();
    if !digits.is_empty() && text[i + digits.len()..].starts_with('.') { digits.parse().ok() } else { None }
}

fn sources_version_in(dir: &Path) -> Option<u32> {
    std::fs::read_dir(dir).ok()?.flatten().find_map(|e| {
        let n = e.file_name().to_string_lossy().into_owned();
        if n.starts_with("Pharo") && n.ends_with(".sources") { major_after_pharo(&n) } else { None }
    })
}

/// Pharo major version of an image: the `PharoNN.x-….sources` file next to it (cheap), then next to the VM, last a
/// one-time `vm --headless image printVersion` probe (works on Pharo 13 and 14). `None` when nothing answers.
pub fn detect_version(vm: &Path, image: &Path) -> Option<u32> {
    if let Some(v) = image.parent().and_then(sources_version_in) {
        return Some(v);
    }
    // the sources of a downloaded VM sit beside it (or a few levels up: <pharo>/pharo-vm/Pharo.app/Contents/MacOS/Pharo)
    for up in vm.ancestors().skip(1).take(6) {
        if let Some(v) = sources_version_in(up) {
            return Some(v);
        }
    }
    // `printVersion --release` prints "14.0" / "13.1" on both; the plain form prints "[version] 'Pharo14.0.0' ..."
    let out = Command::new(vm).arg("--headless").arg(image).arg("printVersion").arg("--release").stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
    String::from_utf8_lossy(&out.stdout).lines().find_map(|l| {
        let l = l.trim();
        let digits: String = l.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() && l[digits.len()..].starts_with('.') { digits.parse().ok() } else { major_after_pharo(l) }
    })
}

/// A template: the VM, the prepared image and the Pharo major version they are.
#[derive(Debug, Clone)]
pub struct Template {
    pub vm: PathBuf,
    pub image: PathBuf,
    pub version: u32,
}

fn read_json(path: &Path) -> Option<Value> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

fn template_from(v: &Value) -> Option<Template> {
    let vm = PathBuf::from(v["vm"].as_str()?);
    let image = PathBuf::from(v["image"].as_str()?);
    // a config written before 0.8 has no "pharo": found out now, never assumed
    let version = v["pharo"].as_u64().map(|n| n as u32).or_else(|| detect_version(&vm, &image))?;
    Some(Template { vm, image, version })
}

/// The default template (`config.json`, directory `pharo/`), if present.
pub fn default_template(home: &Path) -> Option<Template> {
    template_from(&read_json(&home.join("config.json"))?)
}

/// Template of a Pharo version: the default one when it is that version, else the one made by `init --pharo N`
/// (`config-N.json`, directory `pharoN/`). `None` for the version means the default template.
pub fn template(home: &Path, version: Option<u32>) -> Option<Template> {
    let default = default_template(home);
    match version {
        None => default,
        Some(n) if default.as_ref().map(|t| t.version) == Some(n) => default,
        Some(n) => template_from(&read_json(&home.join(format!("config-{}.json", n)))?).filter(|t| t.version == n),
    }
}

/// Arguments after the image that run `script` and keep the VM alive: Pharo before 14 quits after `st` unless told
/// `--no-quit`; Pharo 14 (Clap command line) keeps running by default and rejects `--no-quit`.
pub fn st_serve_args(version: u32, script: &Path) -> Vec<std::ffi::OsString> {
    let mut a: Vec<std::ffi::OsString> = vec!["st".into(), script.into()];
    if version < 14 {
        a.push("--no-quit".into());
    }
    a
}

pub fn init(home: &Path, force: bool, local: Option<(PathBuf, PathBuf)>, pharo: Option<u32>) -> Result<Value, String> {
    // Which version, and is it the default template (home/pharo, config.json) or an additional one (home/pharoN, config-N.json)?
    let existing_default = default_template(home);
    let local_version = match &local {
        Some((lvm, limg)) => Some(detect_version(lvm, limg).ok_or_else(|| format!("cannot tell which Pharo version {} is (no PharoNN.x-….sources file next to it, and `printVersion` gave nothing)", limg.display()))?),
        None => None,
    };
    if let (Some(want), Some(have)) = (pharo, local_version) {
        if want != have {
            return Err(format!("--pharo {} given, but {} is a Pharo {} image", want, local.as_ref().map(|l| l.1.display().to_string()).unwrap_or_default(), have));
        }
    }
    let version = pharo.or(local_version).or(existing_default.as_ref().map(|t| t.version)).unwrap_or(DEFAULT_PHARO);
    if !SUPPORTED_PHARO.contains(&version) {
        return Err(format!("Pharo {} is not supported (stlive supports {})", version, SUPPORTED_PHARO.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" and ")));
    }
    let is_default = existing_default.as_ref().map(|t| t.version == version).unwrap_or(true);
    let (pharo_dir, config_path) = if is_default {
        (home.join("pharo"), home.join("config.json"))
    } else {
        (home.join(format!("pharo{}", version)), home.join(format!("config-{}.json", version)))
    };
    // scratch files of the build: in the home for the default template (as before), in the version's directory otherwise
    let aux = |name: &str| if is_default { home.join(name) } else { pharo_dir.join(name) };
    std::fs::create_dir_all(&pharo_dir).map_err(|e| e.to_string())?;

    // 1. Pharo VM + base image: a local Pharo (--vm/--image) or get.pharo.org
    if let Some((lvm, limg)) = &local {
        copy_unless_same(limg, &pharo_dir.join("Pharo.image")).map_err(|e| format!("{}: {}", limg.display(), e))?;
        let _ = copy_unless_same(&limg.with_extension("changes"), &pharo_dir.join("Pharo.changes"));
        // the sources file must sit next to the image
        if let Ok(rd) = std::fs::read_dir(limg.parent().unwrap()) {
            for e in rd.flatten() {
                if e.path().extension().map(|x| x == "sources").unwrap_or(false) {
                    let link = pharo_dir.join(e.file_name());
                    if !link.exists() { crate::platform::link_or_copy(&e.path(), &link); }
                }
            }
        }
        let _ = lvm;
    } else if force || !pharo_dir.join("Pharo.image").exists() || find_vm(&pharo_dir).is_none() {
        eprintln!("stlive: downloading Pharo {} (VM + image, ~100 MB) from get.pharo.org ...", version);
        crate::platform::download_pharo(&pharo_dir, version)?;
    }
    let vm = match &local {
        Some((lvm, _)) => {
            std::fs::canonicalize(lvm).map_err(|e| format!("{}: {}", lvm.display(), e))?;
            crate::platform::absolute(lvm)
        }
        None => find_vm(&pharo_dir).ok_or("Pharo VM not found after download")?,
    };

    // 2. Sources: the template's copy (home/src, the tree the template image was built from)
    let src = home.join("src");
    write_source_tree(&src)?;
    std::fs::write(home.join("prepare.st"), PREPARE_ST).map_err(|e| e.to_string())?;

    // 3a. Base image + NeoJSON, built once and cached (network only here; re-running `init` after an stlive update is offline)
    let base = pharo_dir.join("Pharo.image");
    let neo_image = pharo_dir.join("neojson.image");
    let marker = pharo_dir.join("neojson.ok");
    let base_id = std::fs::metadata(&base).map(|m| m.len().to_string()).unwrap_or_default();
    let cached = !force && neo_image.exists() && std::fs::read_to_string(&marker).map(|t| t.trim() == base_id).unwrap_or(false);
    let mut neo_warning: Option<String> = None;
    if !cached {
        let _ = std::fs::remove_file(&marker);
        std::fs::copy(&base, &neo_image).map_err(|e| e.to_string())?;
        let _ = std::fs::copy(pharo_dir.join("Pharo.changes"), pharo_dir.join("neojson.changes"));
        std::fs::write(home.join("neojson.st"), NEOJSON_ST).map_err(|e| e.to_string())?;
        eprintln!("stlive: loading NeoJSON (once; cached as {}) ...", neo_image.display());
        let _ = Command::new(&vm)
            .arg("--headless")
            .arg(&neo_image)
            .arg("st")
            .arg(home.join("neojson.st"))
            .stdout(std::fs::File::create(aux("neojson.log")).map(Stdio::from).unwrap_or_else(|_| Stdio::null()))
            .stderr(Stdio::null())
            .status();
        let log = std::fs::read_to_string(aux("neojson.log")).unwrap_or_default();
        match log.lines().find(|l| l.contains("stlive: NeoJSON not loaded")) {
            Some(w) => neo_warning = Some(w.to_string()),
            None => { let _ = std::fs::write(&marker, &base_id); }
        }
    }
    // 3b. Image = (base + NeoJSON) + StLive package
    let image = pharo_dir.join("stlive.image");
    std::fs::copy(&neo_image, &image).map_err(|e| e.to_string())?;
    let _ = std::fs::copy(pharo_dir.join(if neo_image.with_extension("changes").exists() { "neojson.changes" } else { "Pharo.changes" }), pharo_dir.join("stlive.changes"));
    eprintln!("stlive: loading the StLive package into the image ...");
    let packages_file = if is_default { home.join("system-packages.txt") } else { home.join(format!("system-packages-{}.txt", version)) };
    run(
        Command::new(&vm)
            .arg("--headless")
            .arg(&image)
            .arg("st")
            .arg(home.join("prepare.st"))
            .env("STLIVE_SRC", &src)
            .env("STLIVE_PACKAGES_FILE", &packages_file)
            .env("STLIVE_SOURCES_HASH", sources_hash())
            .stdout(std::fs::File::create(aux("prepare.log")).map(Stdio::from).unwrap_or_else(|_| Stdio::null()))
            .stderr(Stdio::null()),
        "image preparation",
    )?;
    // A syntax error in the sources must not produce an "initialised" image without a working server.
    if let Ok(log) = std::fs::read_to_string(aux("prepare.log")) {
        let bad: Vec<&str> = log.lines().filter(|l| l.contains("Syntax Error") || l.contains("Missing opener") || l.contains("did not understand") || l.contains("Error:") || l.contains("Undeclared")).take(5).collect();
        if !bad.is_empty() {
            return Err(format!("loading the StLive package into the image reported problems (see {}):\n{}", aux("prepare.log").display(), bad.join("\n")));
        }
    }
    if let Some(w) = neo_warning {
        eprintln!("stlive: warning – {} (JSON for applications will not be available; run `stlive init` again with network access)", w);
    }

    // 4. Remember
    let cfg = json!({"vm": vm, "image": image, "pharo": version});
    std::fs::write(&config_path, cfg.to_string()).map_err(|e| e.to_string())?;
    let next = if is_default { "stlive start".to_string() } else { format!("stlive start --pharo {}", version) };
    Ok(json!({"home": home, "vm": vm, "template_image": image, "pharo": version, "default": is_default, "next": next}))
}

/// Give an instance its own copy of the template image (so `image save` never touches the template).
pub fn instance_image(template_image: &Path, dest_dir: &Path, name: &str) -> Result<PathBuf, String> {
    let dest = dest_dir.join(format!("{}.image", name));
    if dest.exists() {
        return Ok(dest);
    }
    std::fs::create_dir_all(dest_dir).map_err(|e| e.to_string())?;
    std::fs::copy(template_image, &dest).map_err(|e| e.to_string())?;
    let _ = std::fs::copy(template_image.with_extension("changes"), dest.with_extension("changes"));
    // the sources file must sit next to the image
    if let Ok(rd) = std::fs::read_dir(template_image.parent().unwrap()) {
        for e in rd.flatten() {
            if e.path().extension().map(|x| x == "sources").unwrap_or(false) {
                let link = dest_dir.join(e.file_name());
                if !link.exists() {
                    crate::platform::link_or_copy(&e.path(), &link);
                }
            }
        }
    }
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_prefers_stlive_home_then_stlive_dir_then_default() {
        assert_eq!(home_dir_from(Some("/h"), Some("/d")), PathBuf::from("/h"));
        assert_eq!(home_dir_from(None, Some("/d")), PathBuf::from("/d"));
        assert_eq!(home_dir_from(Some(""), Some("/d")), PathBuf::from("/d"));
        assert_eq!(home_dir_from(None, None), crate::platform::home().join(".stlive"));
        assert_eq!(home_dir_from(None, Some(" ")), crate::platform::home().join(".stlive"));
    }

    #[test]
    fn version_is_read_from_sources_names_and_print_version_lines() {
        assert_eq!(major_after_pharo("Pharo14.0-64bit-e649a7f.sources"), Some(14));
        assert_eq!(major_after_pharo("Pharo13.1-64bit-d7c6f76.sources"), Some(13));
        assert_eq!(major_after_pharo("[version] 'Pharo13.1.0' 'Pharo-13.1.0+SNAPSHOT'"), Some(13));
        assert_eq!(major_after_pharo("Pharo.image"), None);
        assert_eq!(major_after_pharo("nothing"), None);
    }

    #[test]
    fn the_keep_alive_flag_depends_on_the_pharo_version() {
        let s = Path::new("serve.st");
        let args = |v| st_serve_args(v, s).iter().map(|a| a.to_string_lossy().into_owned()).collect::<Vec<_>>();
        assert_eq!(args(13), ["st", "serve.st", "--no-quit"]);
        assert_eq!(args(14), ["st", "serve.st"]);
    }

    #[test]
    fn a_template_config_without_pharo_is_detected_lazily() {
        let home = std::env::temp_dir().join(format!("stlive-tpl-test-{}", std::process::id()));
        let dir = home.join("pharo");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Pharo13.1-64bit-abc.sources"), b"s").unwrap();
        std::fs::write(home.join("config.json"), json!({"vm": dir.join("vm"), "image": dir.join("stlive.image")}).to_string()).unwrap();
        assert_eq!(default_template(&home).unwrap().version, 13);
        assert_eq!(template(&home, Some(13)).unwrap().version, 13);
        assert!(template(&home, Some(14)).is_none());
        // an additional version beside the default one
        let dir14 = home.join("pharo14");
        std::fs::create_dir_all(&dir14).unwrap();
        std::fs::write(home.join("config-14.json"), json!({"vm": dir14.join("vm"), "image": dir14.join("stlive.image"), "pharo": 14}).to_string()).unwrap();
        assert_eq!(template(&home, Some(14)).unwrap().version, 14);
        assert_eq!(template(&home, None).unwrap().version, 13);
        std::fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn init_with_the_template_base_image_as_source_does_not_truncate_it() {
        // `stlive init --image <home>/pharo/Pharo.image` (the file init itself would write) must keep the image intact.
        let home = std::env::temp_dir().join(format!("stlive-init-self-{}", std::process::id()));
        let pharo = home.join("pharo");
        std::fs::create_dir_all(&pharo).unwrap();
        let base = pharo.join("Pharo.image");
        std::fs::write(&base, b"image bytes").unwrap();
        std::fs::write(pharo.join("Pharo.changes"), b"changes").unwrap();
        let vm = pharo.join("no-such-vm");
        // the VM does not exist, so init fails after the copy step; the base image must still be intact
        std::fs::write(pharo.join("Pharo13.1-64bit-abc.sources"), b"s").unwrap();
        let r = init(&home, false, Some((vm, base.clone())), None);
        assert!(r.is_err());
        assert_eq!(std::fs::read(&base).unwrap(), b"image bytes");
        std::fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn sources_are_written_once_under_the_hash_of_the_binary() {
        let home = std::env::temp_dir().join(format!("stlive-src-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let dir = write_sources(&home).unwrap();
        assert_eq!(dir, home.join(format!("src-{}", sources_hash())));
        let written = std::fs::read_to_string(dir.join("StLive/StLiveCommands.class.st")).unwrap();
        assert_eq!(written, include_str!("../smalltalk/src/StLive/StLiveCommands.class.st"));
        assert!(dir.join("StLive-Tests/StLiveUndeclaredReferencesTest.class.st").exists());
        // the tree is in place: a second call answers the same directory and leaves it alone
        std::fs::write(dir.join("marker"), b"kept").unwrap();
        assert_eq!(write_sources(&home).unwrap(), dir);
        assert_eq!(std::fs::read(dir.join("marker")).unwrap(), b"kept");
        // no temporary tree is left next to it
        assert!(std::fs::read_dir(&home).unwrap().flatten().all(|e| !e.file_name().to_string_lossy().contains(".tmp-")));
        std::fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn copy_onto_itself_keeps_the_file() {
        let dir = std::env::temp_dir().join(format!("stlive-init-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("Pharo.image");
        std::fs::write(&f, b"image bytes").unwrap();
        copy_unless_same(&f, &f).unwrap();
        // the same file reached through another path (dir/./Pharo.image)
        copy_unless_same(&dir.join(".").join("Pharo.image"), &f).unwrap();
        assert_eq!(std::fs::read(&f).unwrap(), b"image bytes");
        // different files are still copied
        let g = dir.join("copy.image");
        copy_unless_same(&f, &g).unwrap();
        assert_eq!(std::fs::read(&g).unwrap(), b"image bytes");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
