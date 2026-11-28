//! Sets `GRANIT_GIT_HASH` and `GRANIT_GIT_DIRTY` for the about dialog, as
//! the Tauri app's `src-tauri/build.rs` does.

use std::process::Command;

fn main() {
    emit_git_rerun_hints();
    println!("cargo:rustc-env=GRANIT_GIT_HASH={}", git_hash());
    println!("cargo:rustc-env=GRANIT_GIT_DIRTY={}", git_dirty());
}

/// The current commit hash: `git rev-parse HEAD`, else `GITHUB_SHA` (set in
/// GitHub Actions), else `unknown`.
fn git_hash() -> String {
    git(&["rev-parse", "HEAD"])
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("GITHUB_SHA").ok())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Whether the working tree has staged, unstaged or untracked changes.
fn git_dirty() -> bool {
    git(&["status", "--porcelain"]).is_some_and(|out| !out.is_empty())
}

/// Re-run when the git state changes: checkouts and commits (`HEAD`, the
/// branch ref), staging (`index`) and `git gc` / fetch (`packed-refs`).
fn emit_git_rerun_hints() {
    let Some(git_dir) = git(&["rev-parse", "--absolute-git-dir"]) else {
        return;
    };
    let git_dir = std::path::Path::new(&git_dir);
    for file in ["HEAD", "index", "packed-refs"] {
        println!("cargo:rerun-if-changed={}", git_dir.join(file).display());
    }
    if let Some(branch_ref) = git(&["symbolic-ref", "HEAD"]).filter(|s| !s.is_empty()) {
        println!(
            "cargo:rerun-if-changed={}",
            git_dir.join(branch_ref).display()
        );
    }
}

/// The trimmed output of a successful `git` command.
fn git(args: &[&str]) -> Option<String> {
    Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
}
