use std::fs;
use latentspace::ai::validate;

#[test]
fn all_scripts_pass_validation() {
    let entries = fs::read_dir("scripts").expect("scripts/ directory should exist");

    let mut script_count = 0;
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "lua") {
            script_count += 1;
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("Failed to read {}: {e}", path.display()));
            let result = validate::validate_source(&source);
            for check in &result.checks {
                assert!(
                    check.passed,
                    "Script {} failed check '{}': {}",
                    path.display(),
                    check.name,
                    check.error.as_deref().unwrap_or("unknown error")
                );
            }
        }
    }

    assert!(
        script_count >= 2,
        "Expected at least 2 scripts in scripts/, found {script_count}"
    );
}
