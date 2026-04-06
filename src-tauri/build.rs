use git2::{Repository, Status, StatusOptions};

fn main() {
    emit_git_metadata();
    tauri_build::build();
}

fn emit_git_metadata() {
    let repo = Repository::discover(".");

    let (git_hash, git_dirty) = match repo {
        Ok(repo) => {
            emit_git_rerun_hints(&repo);
            let git_hash = repo
                .head()
                .ok()
                .and_then(|head| head.target())
                .map(|oid| oid.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            let git_dirty = repo_is_dirty(&repo).unwrap_or(false);
            (git_hash, git_dirty)
        }
        Err(_) => ("unknown".to_string(), false),
    };

    println!("cargo:rustc-env=GRANIT_GIT_HASH={git_hash}");
    println!("cargo:rustc-env=GRANIT_GIT_DIRTY={git_dirty}");
}

fn repo_is_dirty(repo: &Repository) -> Result<bool, git2::Error> {
    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false)
        .renames_head_to_index(true);

    let statuses = repo.statuses(Some(&mut options))?;
    Ok(statuses
        .iter()
        .any(|entry| entry.status() != Status::CURRENT))
}

fn emit_git_rerun_hints(repo: &Repository) {
    let git_dir = repo.path();
    println!("cargo:rerun-if-changed={}", git_dir.join("HEAD").display());
    println!("cargo:rerun-if-changed={}", git_dir.join("index").display());
    println!(
        "cargo:rerun-if-changed={}",
        git_dir.join("packed-refs").display()
    );

    if let Ok(head) = repo.head() {
        if let Some(name) = head.name() {
            println!("cargo:rerun-if-changed={}", git_dir.join(name).display());
        }
    }

    if let Some(workdir) = repo.workdir() {
        for path in [
            workdir.join("Cargo.toml"),
            workdir.join("Trunk.toml"),
            workdir.join("index.html"),
            workdir.join("styles.css"),
            workdir.join("daisyui-theme.mjs"),
            workdir.join("daisyui.mjs"),
            workdir.join("src"),
            workdir.join("public"),
            workdir.join("granit-types/src"),
            workdir.join("src-tauri/build.rs"),
            workdir.join("src-tauri/Cargo.toml"),
            workdir.join("src-tauri/src"),
        ] {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}
