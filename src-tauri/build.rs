use std::process::Command;

fn main() {
    emit_git_metadata();
    tauri_build::build();
}

fn emit_git_metadata() {
    emit_git_rerun_hints();

    let git_hash = git_hash();
    let git_dirty = git_dirty();

    println!("cargo:rustc-env=GRANIT_GIT_HASH={git_hash}");
    println!("cargo:rustc-env=GRANIT_GIT_DIRTY={git_dirty}");
}

/// Get the current commit hash.
///
/// Tries `git rev-parse HEAD` first, then falls back to the `GITHUB_SHA`
/// environment variable (set automatically in GitHub Actions).
fn git_hash() -> String {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("GITHUB_SHA").ok())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Check if the working tree has uncommitted changes.
///
/// Uses `git status --porcelain` which produces output only when there are
/// staged, unstaged, or untracked changes.
fn git_dirty() -> bool {
    Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

/// Tell Cargo when to re-run this build script.
///
/// We watch git state files so the hash and dirty flag stay fresh after
/// commits, merges, checkouts, and staging changes.
fn emit_git_rerun_hints() {
    // Find the .git directory (works from src-tauri/ subdirectory)
    let git_dir = Command::new("git")
        .args(["rev-parse", "--absolute-git-dir"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    if let Some(git_dir) = git_dir {
        let git = std::path::Path::new(&git_dir);
        // HEAD changes on checkout / commit
        println!("cargo:rerun-if-changed={}", git.join("HEAD").display());
        // index changes on `git add`
        println!("cargo:rerun-if-changed={}", git.join("index").display());
        // packed-refs changes on `git gc` / fetch
        println!(
            "cargo:rerun-if-changed={}",
            git.join("packed-refs").display()
        );
        // The current branch ref file changes on commit
        if let Some(branch_ref) = current_branch_ref() {
            println!("cargo:rerun-if-changed={}", git.join(branch_ref).display());
        }
    }
}

/// Get the current branch ref path (e.g. "refs/heads/main"), if on a branch.
fn current_branch_ref() -> Option<String> {
    Command::new("git")
        .args(["symbolic-ref", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}
