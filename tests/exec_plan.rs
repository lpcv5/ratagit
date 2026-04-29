use std::fs;
use std::path::Path;
use std::process::Command;

const REQUIRED_HEADINGS: &[&str] = &[
    "## Active Slice",
    "## Problem",
    "## Smallest Slice",
    "## Non-Goals",
    "## Expected Files",
    "## Tests",
    "## Harness Decision",
    "## Validation",
    "## Completion Evidence",
];

#[test]
fn active_exec_plan_has_agent_readable_fields() {
    let plan = read_exec_plan();

    for heading in REQUIRED_HEADINGS {
        assert!(
            plan.contains(heading),
            "docs/EXEC_PLAN.md is missing required heading `{heading}`. \
             Update the active slice before changing implementation files."
        );
    }

    assert_section_has_content(&plan, "## Active Slice");
    assert_section_has_content(&plan, "## Problem");
    assert_section_has_content(&plan, "## Smallest Slice");
    assert_section_has_content(&plan, "## Non-Goals");
    assert_section_has_content(&plan, "## Expected Files");
    assert_section_has_content(&plan, "## Tests");
    assert_section_has_content(&plan, "## Harness Decision");
    assert_section_has_content(&plan, "## Validation");
}

#[test]
fn completed_exec_plan_records_validation_evidence() {
    let plan = read_exec_plan();
    let status = section_body(&plan, "## Active Slice");

    if status.contains("Status: completed") {
        let completion = section_body(&plan, "## Completion Evidence");
        assert!(
            !completion.contains("TBD") && !completion.trim().is_empty(),
            "docs/EXEC_PLAN.md is marked completed but lacks completion evidence. \
             Record what changed and which validation commands passed."
        );

        let validation = section_body(&plan, "## Validation");
        assert!(
            validation.contains("cargo fmt") && validation.contains("cargo test"),
            "docs/EXEC_PLAN.md is marked completed but validation does not include \
             the standard formatting and test checks."
        );
    }
}

#[test]
fn dirty_worktree_changes_include_exec_plan_update() {
    let Some(paths) = changed_workspace_paths() else {
        return;
    };

    if paths.is_empty() {
        return;
    }

    assert!(
        paths.iter().any(|path| path == "docs/EXEC_PLAN.md"),
        "workspace has uncommitted changes but docs/EXEC_PLAN.md is unchanged. \
         Update the active exec plan before implementation edits. Changed paths: {paths:?}"
    );
}

fn read_exec_plan() -> String {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = manifest_dir.join("docs").join("EXEC_PLAN.md");
    fs::read_to_string(&path).expect("docs/EXEC_PLAN.md should be readable")
}

fn changed_workspace_paths() -> Option<Vec<String>> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=all", "--"])
        .current_dir(manifest_dir)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Some(stdout.lines().filter_map(status_path).collect())
}

fn status_path(line: &str) -> Option<String> {
    if line.len() < 4 {
        return None;
    }

    let path = &line[3..];
    let path = path.rsplit_once(" -> ").map_or(path, |(_, to)| to);
    Some(path.replace('\\', "/"))
}

fn assert_section_has_content(plan: &str, heading: &str) {
    let body = section_body(plan, heading);
    assert!(
        !body.trim().is_empty(),
        "`{heading}` in docs/EXEC_PLAN.md must describe the active slice."
    );
}

fn section_body<'a>(plan: &'a str, heading: &str) -> &'a str {
    let start = plan
        .find(heading)
        .unwrap_or_else(|| panic!("missing heading `{heading}`"));
    let after_heading = start + heading.len();
    let rest = &plan[after_heading..];
    let end = rest.find("\n## ").unwrap_or(rest.len());
    &rest[..end]
}
