# Babel segment modules

Babel's native Rust backend supports versioned, opt-in segments made of Babel
source files. This implements the first stage of issue #48 without breaking
legacy single-file programs or changing primitive builtin names.

```babel
use core.text
use system.process
print core_text_caps "hello"
print asbool field system_process_run "true" push array _ boxstr "ignored" "ok"
```

Run `cargo run -- run examples/segments_demo.babel`.

Imports use `use segment.module` on their own line. Exported Babel names
start with `segment_module_`, allowing two modules to export similar
operations without collisions or adding syntax to normal expression calls.
The compiler reads the versioned `segments/<segment>/segment.toml`,
then expands the named `.babel` source. Recursive imports are deduplicated;
cycles and undeclared modules are compilation errors. Only explicitly imported
modules are expanded, and the native Rust backend links feature-specific
runtimes on use.

By default, segment files live beside the source under `segments/`; bundled
versions are embedded in the compiler. `BABEL_SEGMENTS_PATH` can override
modules and manifests with a local directory. The resolver does not download
dependencies. `segments/catalog.json` documents public names for code
generation tools.

Current modules: `core.text`, `core.data`, `system.fs`,
`system.process`, `system.terminal`, `network.http`,
`storage.toml`, `ai.chat`, `desktop.gui`, `desktop.browser`,
and `media.audio`.

This is the initial modularization, not complete isolation of builtin
namespaces. Native functions remain accessible for compatibility.
The AIR/LLVM compiler and Babel-to-C bootstrap do not yet resolve imports.

Babel is a language/compiler repository. Applications such as browsers belong in
independent projects; `desktop.browser` is a reusable module, not a bundled app.
