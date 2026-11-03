use babel_compiler::{compile, modules::{format_root, resolve_source}};
use std::fs;
use std::path::Path;

#[test]
fn imports_from_two_segments_compile_without_linking_gui() {
    let source = include_str!("../examples/segments_demo.babel");
    let expanded = resolve_source(Path::new("examples/segments_demo.babel"), source).unwrap();
    assert_eq!(expanded.matches("# module core.text").count(), 1);
    assert_eq!(expanded.matches("# module system.process").count(), 1);
    let generated = compile(&expanded).unwrap().rust;
    assert!(generated.contains("fn babel_core_text_caps("));
    assert!(generated.contains("fn babel_system_process_run("));
    assert!(!generated.contains("fn babel_gui("));
    assert!(!generated.contains("fn babel_chat("));
}

#[test]
fn import_cycle_and_undeclared_module_fail() {
    let dir = std::env::temp_dir().join(format!("babel-segments-{}",std::process::id()));
    let folder = dir.join("segments/core");
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("segment.toml"), "name = \"core\"\nversion = \"1\"\nmodules = [\"text\", \"data\"]\ndependencies = []\n").unwrap();
    fs::write(folder.join("text.babel"), "use core.data\ncore_text_caps x = upper x\n").unwrap();
    fs::write(folder.join("data.babel"), "use core.text\ncore_data_text x = boxstr x\n").unwrap();
    let file = dir.join("main.babel");
    let err = resolve_source(&file, "use core.text\nprint \"hi\"\n").unwrap_err();
    assert!(err.contains("cycle"), "{err}");
    let err = resolve_source(&file, "use core.unlisted\n").unwrap_err();
    assert!(err.contains("not listed"), "{err}");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn duplicate_imports_do_not_duplicate_definitions() {
    let src = "use core.text\nuse core.text\nprint core_text_caps \"Hi\"\n";
    let loaded = resolve_source(Path::new("test.babel"), src).unwrap();
    assert_eq!(loaded.matches("# module core.text").count(), 1);
    assert!(compile(&loaded).is_ok());
    assert!(resolve_source(Path::new("test.babel"), "use ../../x\n").is_err());
    assert!(format_root(src).unwrap().starts_with("use core.text\nuse core.text\n"));
}

#[test]
fn browser_is_a_babel_app() {
    let source = include_str!("../apps/browser/main.babel");
    let expanded = resolve_source(Path::new("apps/browser/main.babel"), source).unwrap();
    let generated = compile(&expanded).unwrap().rust;
    assert!(generated.contains("fn babel_desktop_browser_open("));
    assert!(generated.contains("--extensions-on-chrome-urls"));
}
