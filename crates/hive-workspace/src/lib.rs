//! Workspaces and git.
//!
//! Repository discovery and branch names come from reading `.git` files
//! directly, so they are cheap enough to refresh periodically. The few
//! operations that change a repository (adding and removing worktrees) and
//! the dirty check invoke `git` with an argv, never through a shell.

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A git working tree containing some path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    /// Top of the working tree that contains the path. For a linked worktree
    /// this is the worktree's own directory.
    pub worktree_root: PathBuf,
    /// This working tree's git dir: `<root>/.git`, or
    /// `<main>/.git/worktrees/<name>` for a linked worktree.
    pub git_dir: PathBuf,
    /// Root of the main working tree. Identifies the repository: every
    /// linked worktree of a repository shares it.
    pub main_root: PathBuf,
}

impl Repo {
    pub fn is_linked_worktree(&self) -> bool {
        self.worktree_root != self.main_root
    }

    /// The checked-out branch, or a short commit id when HEAD is detached.
    pub fn branch(&self) -> Option<String> {
        read_branch(&self.git_dir)
    }
}

/// Finds the working tree containing `path` (which must exist). Returns
/// `None` outside any repository. Like git, the search stops at a filesystem
/// boundary.
pub fn discover(path: &Path) -> Option<Repo> {
    let path = path.canonicalize().ok()?;
    let device = fs::metadata(&path).ok()?.dev();
    for dir in path.ancestors() {
        if fs::metadata(dir).ok()?.dev() != device {
            return None;
        }
        let dot_git = dir.join(".git");
        let Ok(meta) = fs::metadata(&dot_git) else {
            continue;
        };
        let git_dir = if meta.is_dir() {
            dot_git
        } else {
            // A linked worktree or submodule: `.git` is a file naming the
            // real git dir, possibly relative to the working tree.
            let text = fs::read_to_string(&dot_git).ok()?;
            let target = text.strip_prefix("gitdir:")?.trim();
            dir.join(target).canonicalize().ok()?
        };
        if !git_dir.join("HEAD").is_file() {
            continue;
        }
        let common_dir = match fs::read_to_string(git_dir.join("commondir")) {
            Ok(rel) => git_dir.join(rel.trim()).canonicalize().ok()?,
            Err(_) => git_dir.clone(),
        };
        // A linked worktree's common dir is `<main>/.git`. Anything else
        // (a main tree, or a submodule's `.git/modules/x`) is its own root.
        let linked = !meta.is_dir()
            && common_dir != git_dir
            && common_dir.file_name().is_some_and(|n| n == ".git");
        let main_root = match common_dir.parent() {
            Some(main) if linked => main.to_path_buf(),
            _ => dir.to_path_buf(),
        };
        return Some(Repo {
            worktree_root: dir.to_path_buf(),
            git_dir,
            main_root,
        });
    }
    None
}

/// Reads `<git_dir>/HEAD`: the branch name, or the first 7 hex digits of a
/// detached HEAD.
pub fn read_branch(git_dir: &Path) -> Option<String> {
    let head = fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let head = head.trim();
    if let Some(r) = head.strip_prefix("ref:") {
        let r = r.trim();
        return Some(r.strip_prefix("refs/heads/").unwrap_or(r).to_string());
    }
    (head.len() >= 7 && head.bytes().all(|b| b.is_ascii_hexdigit())).then(|| head[..7].to_string())
}

/// The path that identifies the workspace `cwd` belongs to: its
/// repository's main root, or `cwd` itself outside git.
pub fn workspace_key(cwd: &Path) -> PathBuf {
    match discover(cwd) {
        Some(repo) => repo.main_root,
        None => cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf()),
    }
}

/// Picks the workspace whose root is the deepest ancestor of (or equal to)
/// `key`.
pub fn best_match<'a, I>(workspaces: I, key: &Path) -> Option<u64>
where
    I: IntoIterator<Item = (u64, &'a Path)>,
{
    workspaces
        .into_iter()
        .filter(|(_, root)| key.starts_with(root))
        .max_by_key(|(_, root)| root.components().count())
        .map(|(id, _)| id)
}

/// The Obsidian vault (a directory holding `.obsidian/`) for a session in
/// `cwd` under workspace `root`: `cwd` or an ancestor up to `root`, else a
/// direct subdirectory of `root` (e.g. `docs/`, `vault/`), first by name.
pub fn find_obsidian_vault(cwd: &Path, root: &Path) -> Option<PathBuf> {
    let is_vault = |d: &Path| d.join(".obsidian").is_dir();
    let mut dir = Some(cwd);
    while let Some(d) = dir {
        if is_vault(d) {
            return Some(d.to_path_buf());
        }
        if d == root || !d.starts_with(root) {
            break;
        }
        dir = d.parent();
    }
    let mut subdirs: Vec<PathBuf> = std::fs::read_dir(root)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    subdirs.sort();
    subdirs.into_iter().find(|d| is_vault(d))
}

/// A display name for a workspace rooted at `root`.
pub fn default_name(root: &Path) -> String {
    root.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string())
}

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("could not run git: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("git {command} failed: {stderr}")]
    Failed { command: String, stderr: String },
    #[error("{} has uncommitted changes", .0.display())]
    Dirty(PathBuf),
    #[error("{0}")]
    Refused(String),
}

fn git(dir: &Path, args: &[&str]) -> Result<String, GitError> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C")
        .output()?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(GitError::Failed {
            command: args.first().copied().unwrap_or_default().to_string(),
            stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        })
    }
}

/// True if the working tree has staged, unstaged or untracked changes
/// (ignored files don't count).
pub fn is_dirty(worktree_root: &Path) -> Result<bool, GitError> {
    Ok(!git(worktree_root, &["status", "--porcelain"])?
        .trim()
        .is_empty())
}

/// Rejects names git wouldn't accept as a branch.
pub fn check_branch_name(main_root: &Path, branch: &str) -> Result<(), GitError> {
    git(main_root, &["check-ref-format", "--branch", branch])
        .map(drop)
        .map_err(|_| GitError::Refused(format!("`{branch}` is not a valid branch name")))
}

/// Creates a linked worktree at `path` on `branch`, checking out the branch
/// if it exists and creating it from the current HEAD otherwise.
pub fn add_worktree(main_root: &Path, path: &Path, branch: &str) -> Result<(), GitError> {
    check_branch_name(main_root, branch)?;
    let path_arg = path.to_string_lossy();
    let exists = git(
        main_root,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ],
    )
    .is_ok();
    if exists {
        git(main_root, &["worktree", "add", "--", &path_arg, branch])?;
    } else {
        git(
            main_root,
            &["worktree", "add", "-b", branch, "--", &path_arg],
        )?;
    }
    Ok(())
}

/// Removes the linked worktree at `path`. Refuses the main working tree,
/// and refuses a dirty one unless `force`. The branch is never deleted.
pub fn remove_worktree(path: &Path, force: bool) -> Result<(), GitError> {
    let repo = discover(path).ok_or_else(|| {
        GitError::Refused(format!("{} is not in a git repository", path.display()))
    })?;
    if !repo.is_linked_worktree() || repo.worktree_root != path.canonicalize()? {
        return Err(GitError::Refused(format!(
            "{} is not a linked worktree",
            path.display()
        )));
    }
    if !force && is_dirty(&repo.worktree_root)? {
        return Err(GitError::Dirty(repo.worktree_root));
    }
    let root = repo.worktree_root.to_string_lossy().into_owned();
    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.extend(["--", &root]);
    git(&repo.main_root, &args)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Worktree {
    pub path: PathBuf,
    /// `None` when detached.
    pub branch: Option<String>,
    pub main: bool,
}

/// Every working tree of the repository, main first.
pub fn list_worktrees(main_root: &Path) -> Result<Vec<Worktree>, GitError> {
    let out = git(main_root, &["worktree", "list", "--porcelain"])?;
    let mut list: Vec<Worktree> = Vec::new();
    for line in out.lines() {
        if let Some(p) = line.strip_prefix("worktree ") {
            list.push(Worktree {
                path: PathBuf::from(p),
                branch: None,
                main: list.is_empty(),
            });
        } else if let (Some(b), Some(w)) = (line.strip_prefix("branch "), list.last_mut()) {
            w.branch = Some(b.strip_prefix("refs/heads/").unwrap_or(b).to_string());
        }
    }
    Ok(list)
}

/// Turns a branch name into one path component (`hive/fix-ui` → `hive-fix-ui`).
pub fn branch_dir_name(branch: &str) -> String {
    branch
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches(['-', '.'])
        .to_string()
}

/// Directory under the worktree base reserved for one repository:
/// `<name>-<hash of its path>`, so same-named repositories don't collide.
pub fn repo_dir_name(main_root: &Path) -> String {
    // FNV-1a: stable across builds, unlike std's hasher.
    let mut h: u32 = 0x811c_9dc5;
    for b in main_root.as_os_str().as_encoded_bytes() {
        h = (h ^ u32::from(*b)).wrapping_mul(0x0100_0193);
    }
    format!("{}-{h:08x}", branch_dir_name(&default_name(main_root)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(dir: &Path, args: &[&str]) {
        let st = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .status()
            .unwrap();
        assert!(st.success(), "git {args:?}");
    }

    #[test]
    fn obsidian_vaults_are_found_at_or_above_cwd_then_one_level_down() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let deep = root.join("src/deep");
        std::fs::create_dir_all(&deep).unwrap();
        assert_eq!(find_obsidian_vault(&deep, root), None);

        std::fs::create_dir_all(root.join("zz/.obsidian")).unwrap();
        std::fs::create_dir_all(root.join("vault/.obsidian")).unwrap();
        std::fs::create_dir_all(root.join(".hidden/.obsidian")).unwrap();
        assert_eq!(find_obsidian_vault(&deep, root), Some(root.join("vault")));

        std::fs::create_dir_all(root.join("src/.obsidian")).unwrap();
        assert_eq!(find_obsidian_vault(&deep, root), Some(root.join("src")));
    }

    fn repo() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("proj");
        fs::create_dir(&root).unwrap();
        run(&root, &["init", "-q", "-b", "main"]);
        fs::write(root.join("a.txt"), "a").unwrap();
        run(&root, &["add", "."]);
        run(&root, &["commit", "-q", "-m", "init"]);
        let root = root.canonicalize().unwrap();
        (dir, root)
    }

    #[test]
    fn discovers_repo_from_subdirectory_and_reads_branch() {
        let (_d, root) = repo();
        fs::create_dir(root.join("src")).unwrap();
        let r = discover(&root.join("src")).unwrap();
        assert_eq!(r.worktree_root, root);
        assert_eq!(r.main_root, root);
        assert!(!r.is_linked_worktree());
        assert_eq!(r.branch().as_deref(), Some("main"));
        run(&root, &["checkout", "-q", "--detach"]);
        assert_eq!(r.branch().unwrap().len(), 7);
    }

    #[test]
    fn outside_git_the_key_is_the_directory() {
        let dir = tempfile::tempdir().unwrap();
        assert!(discover(dir.path()).is_none());
        assert_eq!(
            workspace_key(dir.path()),
            dir.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn linked_worktree_shares_main_root_and_has_its_own_branch() {
        let (d, root) = repo();
        let wt = d.path().join("wts").join("feature");
        add_worktree(&root, &wt, "hive/feature").unwrap();
        let r = discover(&wt).unwrap();
        assert!(r.is_linked_worktree());
        assert_eq!(r.main_root, root);
        assert_eq!(r.worktree_root, wt.canonicalize().unwrap());
        assert_eq!(r.branch().as_deref(), Some("hive/feature"));
        assert_eq!(workspace_key(&wt), root);

        let list = list_worktrees(&root).unwrap();
        assert_eq!(list.len(), 2);
        assert!(list[0].main);
        assert_eq!(list[1].branch.as_deref(), Some("hive/feature"));

        // An existing branch is checked out rather than recreated.
        run(&root, &["branch", "other"]);
        let wt2 = d.path().join("wts").join("other");
        add_worktree(&root, &wt2, "other").unwrap();
        assert_eq!(discover(&wt2).unwrap().branch().as_deref(), Some("other"));
    }

    #[test]
    fn dirty_worktrees_are_not_removed_without_force_and_branches_survive() {
        let (d, root) = repo();
        let wt = d.path().join("wt");
        add_worktree(&root, &wt, "work").unwrap();
        fs::write(wt.join("new.txt"), "x").unwrap();
        assert!(is_dirty(&wt).unwrap());
        assert!(matches!(
            remove_worktree(&wt, false),
            Err(GitError::Dirty(_))
        ));
        assert!(wt.exists());
        remove_worktree(&wt, true).unwrap();
        assert!(!wt.exists());
        // The branch is kept.
        run(
            &root,
            &["rev-parse", "--verify", "--quiet", "refs/heads/work"],
        );
    }

    #[test]
    fn main_worktree_and_bad_branch_names_are_refused() {
        let (d, root) = repo();
        assert!(matches!(
            remove_worktree(&root, true),
            Err(GitError::Refused(_))
        ));
        assert!(matches!(
            add_worktree(&root, &d.path().join("x"), "bad..name"),
            Err(GitError::Refused(_))
        ));
    }

    #[test]
    fn best_match_prefers_the_deepest_root() {
        let a = PathBuf::from("/src");
        let b = PathBuf::from("/src/app");
        let ws = [(1, a.as_path()), (2, b.as_path())];
        assert_eq!(best_match(ws, Path::new("/src/app/x")), Some(2));
        assert_eq!(best_match(ws, Path::new("/src/lib")), Some(1));
        assert_eq!(best_match(ws, Path::new("/srcx")), None);
    }

    #[test]
    fn names_are_path_safe() {
        assert_eq!(branch_dir_name("hive/fix ui"), "hive-fix-ui");
        let n = repo_dir_name(Path::new("/home/me/my proj"));
        assert!(n.starts_with("my-proj-") && n.len() == "my-proj-".len() + 8);
    }
}
