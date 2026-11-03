//! Deterministic, opt-in Babel segment imports for the native Rust backend.
//! A module's public names are prefixed: core.text -> core_text_*.
//! This keeps today's one-file compiler and builtin aliases source compatible.
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

fn builtin_manifest(segment: &str) -> Option<&'static str> {
    Some(match segment {
        "core" => include_str!("../segments/core/segment.toml"),
        "system" => include_str!("../segments/system/segment.toml"),
        "network" => include_str!("../segments/network/segment.toml"),
        "storage" => include_str!("../segments/storage/segment.toml"),
        "ai" => include_str!("../segments/ai/segment.toml"),
        "desktop" => include_str!("../segments/desktop/segment.toml"),
        "media" => include_str!("../segments/media/segment.toml"),
        _ => return None,
    })
}
fn builtin_module(name: &str) -> Option<&'static str> {
    Some(match name {
        "core.text" => include_str!("../segments/core/text.babel"),
        "core.data" => include_str!("../segments/core/data.babel"),
        "system.fs" => include_str!("../segments/system/fs.babel"),
        "system.process" => include_str!("../segments/system/process.babel"),
        "system.terminal" => include_str!("../segments/system/terminal.babel"),
        "network.http" => include_str!("../segments/network/http.babel"),
        "storage.toml" => include_str!("../segments/storage/toml.babel"),
        "ai.chat" => include_str!("../segments/ai/chat.babel"),
        "desktop.gui" => include_str!("../segments/desktop/gui.babel"),
        "desktop.browser" => include_str!("../segments/desktop/browser.babel"),
        "media.audio" => include_str!("../segments/media/audio.babel"),
        _ => return None,
    })
}
fn import(line: &str) -> Result<Option<&str>, String> {
    let text = line.trim();
    let Some(rest) = text.strip_prefix("use ") else { return Ok(None); };
    let name = rest.split('#').next().unwrap_or("").trim();
    let parts: Vec<_> = name.split('.').collect();
    if parts.len() != 2 || parts.iter().any(|part| {
        part.is_empty() || !part.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    }) || name.split_whitespace().count() != 1 {
        return Err(format!("invalid segment import '{name}' (expected: use segment.module)"));
    }
    Ok(Some(name))
}
fn array_field<'a>(text: &'a str, key: &str) -> Result<Vec<&'a str>, String> {
    let value = field(text, key)?;
    let Some(inner) = value.strip_prefix('[').and_then(|s| s.strip_suffix(']')) else {
        return Err(format!("manifest field {key} must be an array"));
    };
    if inner.trim().is_empty() { return Ok(vec![]); }
    inner.split(',').map(|s| {
        let s = s.trim();
        s.strip_prefix('"').and_then(|s| s.strip_suffix('"'))
            .filter(|s| !s.is_empty())
            .ok_or_else(|| format!("malformed manifest {key} value"))
    }).collect()
}
fn field<'a>(text: &'a str, key: &str) -> Result<&'a str, String> {
    for line in text.lines() {
        let Some((left, right)) = line.split_once('=') else { continue; };
        if left.trim() == key { return Ok(right.trim()); }
    }
    Err(format!("manifest missing {key}"))
}
fn string_field(text: &str, key: &str) -> Result<String, String> {
    field(text, key)?.strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .map(str::to_string)
        .ok_or_else(|| format!("manifest field {key} must be a string"))
}
struct Resolver {
    roots: Vec<PathBuf>,
    emitted: HashSet<String>,
    visiting: Vec<String>,
    output: String,
}
impl Resolver {
    fn lookup(&self, path: &str, fallback: Option<&str>) -> Result<String, String> {
        for root in &self.roots {
            let candidate = root.join(path);
            if candidate.is_file() {
                return fs::read_to_string(&candidate)
                    .map_err(|e| format!("{}: {e}", candidate.display()));
            }
        }
        fallback.map(str::to_owned)
            .ok_or_else(|| format!("unknown segment module {path}"))
    }
    fn module(&mut self, name: &str) -> Result<(), String> {
        if self.visiting.iter().any(|part| part == name) {
            return Err(format!("segment import cycle: {} -> {name}", self.visiting.join(" -> ")));
        }
        if self.emitted.contains(name) { return Ok(()); }
        let (segment, member) = name.split_once('.')
            .ok_or_else(|| format!("invalid module name '{name}'"))?;
        let manifest = self.lookup(&format!("{segment}/segment.toml"), builtin_manifest(segment))?;
        if string_field(&manifest, "name")? != segment {
            return Err(format!("segment manifest name mismatch for '{segment}'"));
        }
        let version = string_field(&manifest, "version")?;
        if version.is_empty() { return Err(format!("missing segment version for {segment}")); }
        if !array_field(&manifest, "modules")?.contains(&member) {
            return Err(format!("module {name} is not listed in {segment}'s manifest"));
        }
        // Dependencies are declared for tooling, while actual dependencies
        // are explicitly imported and checked by the DFS below.
        let _ = array_field(&manifest, "dependencies")?;
        let content = self.lookup(&format!("{segment}/{member}.babel"), builtin_module(name))?;
        self.visiting.push(name.to_string());
        let mut body = String::new();
        for (line, original) in content.lines().enumerate() {
            if let Some(child) = import(original)? {
                self.module(child)?;
                continue;
            }
            let trimmed = original.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                let def_name = trimmed.split_whitespace().next().unwrap_or("");
                if !trimmed.contains('=') || !def_name.starts_with(&format!("{segment}_{member}_")) {
                    return Err(format!("{name}:{}: exports must be prefixed {segment}_{member}_ and modules cannot run top-level statements", line + 1));
                }
            }
            body.push_str(original);
            body.push('\n');
        }
        self.output.push_str(&format!("# module {name} ({version})\n"));
        self.output.push_str(&body);
        self.visiting.pop();
        self.emitted.insert(name.to_string());
        Ok(())
    }
}
/// Resolve imports relative to the source project, or an explicit
/// BABEL_SEGMENTS_PATH. Builtin modules are embedded in babelc for portability.
/// No remote dependencies or implicit code execution occur at import time.
pub fn resolve_source(path: &Path, source: &str) -> Result<String, String> {
    let mut roots = Vec::new();
    if let Some(dir) = std::env::var_os("BABEL_SEGMENTS_PATH") {
        roots.push(PathBuf::from(dir));
    }
    if let Some(parent) = path.parent() {
        roots.push(parent.join("segments"));
    }
    roots.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("segments"));
    let mut resolver = Resolver {
        roots, emitted: HashSet::new(), visiting: vec![], output: String::new(),
    };
    let mut main = String::new();
    for (line, original) in source.lines().enumerate() {
        match import(original).map_err(|e| format!("{}:{}: {e}", path.display(), line + 1))? {
            Some(module) => resolver.module(module)?,
            None => { main.push_str(original); main.push('\n'); }
        }
    }
    resolver.output.push_str(&main);
    Ok(resolver.output)
}
/// Preserve import directives while formatting the root file, rather than
/// feeding them to the one-file Babel expression parser.
pub fn format_root(source: &str) -> Result<String, String> {
    let mut imports = String::new();
    let mut body = String::new();
    for original in source.lines() {
        match import(original)? {
            Some(module) => { imports.push_str("use "); imports.push_str(module); imports.push('\n'); }
            None => { body.push_str(original); body.push('\n'); }
        }
    }
    let formatted = crate::format_source(&body).map_err(|e| e.to_string())?;
    imports.push_str(&formatted);
    Ok(imports)
}
