use std::{ops::Deref, process};

use git2::Repository;
use regex::Regex;
use url::Url;

use crate::DEBUG;

#[derive(clap::Subcommand, Debug, Clone)]
pub enum Command {
    /// Clone a repository
    Clone {
        /// URL for the repository
        #[arg()]
        url: String,
    },

    /// Synchronize local repository with remote repository
    Sync,

    /// Check the status of the repository
    Status,

    /// View commit history
    History,

    /// Stage changes to commit
    Stage {
        /// Pattern for which files should be staged
        #[arg()]
        pattern: String,
    },

    /// Unstage changes that are currently staged
    Unstage {
        /// Pattern for which files should be unstaged
        #[arg()]
        pattern: String,
    },

    /// Clear all local changes to the current branch
    Clear,

    /// Commit currently-staged changes
    Commit {
        /// Commit message
        #[arg()]
        message: String,
    },

    /// Switch to a different branch
    Switch {
        /// Name of the branch to switch to
        #[arg()]
        branch_name: String,
    },

    /// Create a new branch based on the current one
    Branch {
        /// Name for the new branch
        #[arg()]
        branch_name: String,
    },

    Undo,
    Redo,
    Rewrite,

    /// Rebase the current branch on some other branch
    Rebase {
        /// Name for the new branch
        #[arg()]
        other_branch: String,
    },
}

impl Command {
    pub fn perform(self) -> Result<(), String> {
        match self {
            Command::Clone { url } => git(&["clone", &url]),
            Command::Sync => {
                let sync_info = sync()?;
                print_sync_info(sync_info);

                Ok(())
            }
            Command::Status => {
                let branch_name = get_branch_name()?;
                let output = git_with_output(&["status", "--short"])?;

                println!("On branch {}", branch_name);
                println!("{}", output);

                Ok(())
            }
            Command::History => todo!(),
            Command::Stage { pattern } => stage(&pattern),
            Command::Unstage { pattern } => unstage(&pattern),
            Command::Clear => git(&["reset", "--hard"]),
            Command::Commit { message } => {
                git(&["commit", "-m", &message])?;
                let sync_info = sync()?;
                print_sync_info(sync_info);

                Ok(())
            }
            Command::Switch { branch_name } => switch(&branch_name),
            Command::Branch { branch_name } => {
                stash_branch_changes(true)?;
                git(&["checkout", "-b", &branch_name])?;
                Ok(())
            }
            Command::Undo => todo!(),
            Command::Redo => todo!(),
            Command::Rewrite => todo!(), // TODO: Present interactive TUI instead of using system editor
            Command::Rebase { other_branch } => {
                let current_branch = get_branch_name()?;
                switch(&other_branch)?;
                sync()?;
                switch(&current_branch)?;
                git(&["rebase", &other_branch])?; // TODO: Handle merge conflicts somehow

                Ok(())
            }
        }
    }
}

fn print_sync_info((ahead, behind): (usize, usize)) {
    if ahead == 0 && behind == 0 {
        println!("Already up to date");
    } else {
        println!("Pushed {} commits and pulled {} commits", ahead, behind);
    }
}

fn git(args: &[&str]) -> Result<(), String> {
    git_with_output(args).map(|_| ())
}

fn git_with_output(args: &[&str]) -> Result<String, String> {
    process::Command::new("git")
        .args(args)
        .output()
        .map(|o| {
            let out_str = String::from_utf8(o.stdout).unwrap();

            if DEBUG {
                println!(
                    "git {:?}:\n\t{}\n\t{}",
                    args,
                    out_str,
                    String::from_utf8(o.stderr).unwrap()
                );
            }

            out_str
        })
        .map_err(|e| e.to_string())
}

fn sync() -> Result<(usize, usize), String> {
    git(&["fetch"])?;
    let ahead = commits_ahead()?;
    let behind = commits_behind()?;

    git(&["pull", "--rebase"])?;
    git(&["push"])?;

    Ok((ahead, behind))
}

fn switch(branch_name: &str) -> Result<(), String> {
    stash_branch_changes(false)?;
    git(&["checkout", branch_name])?;
    pop_stashed_branch_changes()
}

fn stage(pattern: &str) -> Result<(), String> {
    git(&["add", pattern])
}

fn unstage(pattern: &str) -> Result<(), String> {
    git(&["reset", pattern])
}

fn get_branch_name() -> Result<String, String> {
    git_with_output(&["rev-parse", "--abbrev-ref", "HEAD"]).map(|o| o.trim().to_owned())
}

fn stash_branch_changes(keep: bool) -> Result<(), String> {
    let branch_name = get_branch_name()?;
    let stash_name = stash_name_for_branch(&branch_name);

    stage(".")?;
    if keep {
        git(&["stash", "push", "-k", "-m", &stash_name])?;
        unstage(".")
    } else {
        git(&["stash", "push", "-m", &stash_name])
    }
}

fn pop_stashed_branch_changes() -> Result<(), String> {
    let branch_name = get_branch_name()?;
    let stash_name = stash_name_for_branch(&branch_name);

    let stash = list_stashes()?
        .into_iter()
        .find(|s| s.message.contains(&stash_name));

    if let Some(stash) = &stash {
        git(&["stash", "pop", &stash.reference])?;
        unstage(".")
    } else {
        Ok(())
    }
}

fn stash_name_for_branch(branch_name: &str) -> String {
    format!("gud_local_changes:{}", branch_name)
}

struct Stash {
    pub reference: String,
    pub message: String,
}

fn list_stashes() -> Result<Vec<Stash>, String> {
    let output = git_with_output(&["stash", "list"])?;

    let pattern = Regex::new(r"(stash@\{[0-9]+\}): (On [^\n]*)").unwrap();

    Ok(pattern
        .captures_iter(&output)
        .map(|capt| Stash {
            reference: capt.get(1).unwrap().as_str().to_owned(),
            message: capt.get(2).unwrap().as_str().to_owned(),
        })
        .collect())
}

fn commits_ahead() -> Result<usize, String> {
    let branch_name = get_branch_name()?;
    let output = git_with_output(&[
        "rev-list",
        &format!("origin/{}..{}", branch_name, branch_name),
        "--count",
    ])?;

    output.trim().parse::<usize>().map_err(|e| e.to_string())
}

fn commits_behind() -> Result<usize, String> {
    let branch_name = get_branch_name()?;
    let output = git_with_output(&[
        "rev-list",
        &format!("{}..origin/{}", branch_name, branch_name),
        "--count",
    ])?;

    output.trim().parse::<usize>().map_err(|e| e.to_string())
}

#[test]
fn bar() {
    let pattern = Regex::new(r"(stash@\{[0-9]+\}): (On [^\n]*)").unwrap();
    let str = "stash@{0}: On test_branch: gud_local_changes:test_branch
    stash@{1}: On master: gud_local_changes:master
    ";

    for c in pattern.captures_iter(str) {
        println!("{:?}", c);
    }
}

// pub fn git_credentials_callback(
//     user: &str,
//     user_from_url: Option<&str>,
//     cred: git2::CredentialType,
// ) -> Result<git2::Cred, git2::Error> {
//     let user = user_from_url.unwrap_or("git");

//     if cred.contains(git2::CredentialType::USERNAME) {
//         return git2::Cred::username(user);
//     }

//     match std::env::var("GPM_SSH_KEY") {
//         Ok(k) => {
//             println!(
//                 "authenticate with user {} and private key located in {}",
//                 user, k
//             );
//             git2::Cred::ssh_key(user, None, std::path::Path::new(&k), None)
//         }
//         _ => Err(git2::Error::from_str(
//             "unable to get private key from GPM_SSH_KEY",
//         )),
//     }
// }

// fn get_or_init_repo(remote: &str) -> Result<git2::Repository, git2::Error> {
//     let data_url = match Url::parse(remote) {
//         Ok(data_url) => data_url,
//         Err(e) => panic!("failed to parse url: {}", e),
//     };
//     let path = std::env::current_dir()
//         .unwrap()
//         .join(data_url.host_str().unwrap())
//         .join(&data_url.path()[1..]);

//     if path.exists() {
//         println!("use existing repository {}", path.to_str().unwrap());
//         return git2::Repository::open(path);
//     }

//     let mut callbacks = git2::RemoteCallbacks::new();
//     callbacks.credentials(git_credentials_callback);

//     let mut opts = git2::FetchOptions::new();
//     opts.remote_callbacks(callbacks);
//     opts.download_tags(git2::AutotagOption::All);

//     let mut builder = git2::build::RepoBuilder::new();
//     builder.fetch_options(opts);
//     builder.branch("master");

//     println!(
//         "start cloning repository {} in {}",
//         remote,
//         path.to_str().unwrap()
//     );

//     match builder.clone(remote, &path) {
//         Ok(r) => {
//             println!("repository cloned");

//             Ok(r)
//         }
//         Err(e) => Err(e),
//     }
// }

fn repository_name(url: &str) -> Result<String, ()> {
    let expr = Regex::new(r"([^/.:]+)(?:\.git)?$").unwrap();

    let res = expr
        .captures_iter(&url)
        .nth(0)
        .map(|capt| capt.get(1).map(|res| res.as_str().to_owned()))
        .flatten()
        .ok_or(());

    res.clone()
}

#[test]
fn repository_name_test() {
    let expr = Regex::new(r"([^/.:]+)(?:\.git)?$").unwrap();
    let urls = [
        "git@github.com:brundonsmith/rust_lisp.git",
        "https://github.com/brundonsmith/rust_lisp.git",
        "https://github.com/brundonsmith/rust_lisp",
    ];

    for url in urls {
        assert_eq!(repository_name(url), Ok("rust_lisp".to_owned()));
    }
}
