use git2::Repository;
use regex::Regex;

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
    Rebase, // (on some other branch (guarantee it's the original branch?))
}

impl Command {
    pub fn perform(self) -> Result<(), String> {
        match self {
            Command::Clone { url } => {
                let cwd = std::env::current_dir().unwrap();
                let name = repository_name(&url).unwrap();

                Repository::clone(&url, &cwd.join(name))
                    .map(|_| ())
                    .map_err(|e| format!("failed to clone: {}", e))
            }
            Command::Sync => todo!(),
            Command::Status => todo!(),
            Command::History => todo!(),
            Command::Stage { pattern } => todo!(),
            Command::Unstage { pattern } => todo!(),
            Command::Commit { message } => todo!(),
            Command::Switch { branch_name } => todo!(),
            Command::Branch { branch_name } => todo!(),
            Command::Undo => todo!(),
            Command::Redo => todo!(),
            Command::Rewrite => todo!(),
            Command::Rebase => todo!(),
        }
    }
}

struct RemoteUrl {
    pub url: String,
    pub url_type: RemoteUrlType,
}

enum RemoteUrlType {
    Ssh,    // git@github.com:brundonsmith/rust_lisp.git
    Https,  // https://github.com/brundonsmith/rust_lisp.git
    Github, // https://github.com/brundonsmith/rust_lisp
    Gitlab,
}

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
