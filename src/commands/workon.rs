use error_chain::bail;
use git2::{Cred, Oid, RemoteCallbacks, Repository, RepositoryState, Signature, Status};
use git2::build::CheckoutBuilder;
use git2::FetchOptions;
use regex::Regex;
use std::path::Path;

use crate::errors::*;

const USERNAME: &'static str = "vigoo";
// TODO: configure/get
const NAME: &'static str = "Daniel Vigovszky";
// TODO: configurable
const EMAIL: &'static str = "daniel.vigovszky@gmail.com"; // TODO: configurable

fn get_current_branch(repo: &Repository) -> Result<String> {
    let head = repo.head()?;
    if head.is_branch() {
        let shorthand: Result<&str> = head.shorthand().ok_or("No valid shorthand branch name".into());
        shorthand.map(|s| s.to_string())
    } else {
        bail!("HEAD is not pointing to a branch");
    }
}

fn has_changes(repo: &Repository) -> Result<bool> {
    let statuses: Vec<Status> = repo
        .statuses(None)?
        .iter()
        .map(|entry| { println!("Status of {:?}: {:?}", entry.path(), entry.status()); entry.status() })
        .filter(|status| *status != Status::WT_NEW && *status != Status::IGNORED)
        .collect();

    Ok(!statuses.is_empty())
}

fn stash_changes_if_any(repo: &mut Repository) -> Result<Option<Oid>> {
    let changes = has_changes(repo)?;
    if changes {
        println!("Stashing existing changes");

        let signature = Signature::now(NAME, EMAIL)?;
        repo.stash_save(
            &signature,
            "gut stashing before creating new feature branch",
            None)
            .map(Some)
            .map_err(|e| e.into())
    } else {
        println!("No changes to stash");

        Ok(None)
    }
}

fn unstash_changes(repo: &mut Repository, oid: Oid) -> Result<()> {
    println!("Unstashing changes");

    let mut result_idx: Option<usize> = None;
    let _ = repo.stash_foreach(|idx, _message, stash_oid| {
        if *stash_oid == oid {
            result_idx = Some(idx);
            true
        } else {
            false
        }
    })?;

    match result_idx {
        Some(idx) => {
            let _ = repo.stash_pop(idx, None)?;
            Ok(())
        }
        None =>
            bail!("Could not find stash by oid")
    }
}

fn checkout_branch(repo: &mut Repository, name: &str) -> Result<()> {
    println!("Checking out {}", name);

    let branch_name: &str = &format!("refs/heads/{}", name);
    let branch = repo.revparse_single(branch_name)?;
    let mut opts = CheckoutBuilder::new();
    opts.safe();
    opts.recreate_missing(true);

    let _ = repo.checkout_tree(&branch, Some(&mut opts))?;
    let _ = repo.set_head(branch_name);

    Ok(())
}


fn checkout_master(repo: &mut Repository) -> Result<()> {
    checkout_branch(repo, "master")
}

fn create_branch(repo: &mut Repository, name: &str) -> Result<()> {
    println!("Creating branch {}", name);

    let target_commit = repo.head()?.peel_to_commit()?;
    let _ = repo.branch(name, &target_commit, false)?;
    Ok(())
}

fn pull(repo: &mut Repository) -> Result<()> {
    let mut origin = repo.find_remote("origin")?;

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username, _allowed|
        Cred::ssh_key_from_agent(username.unwrap_or(USERNAME)));

    println!("Fetching remote...");

    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);
    let _ = origin.fetch(&["master"], Some(&mut fetch_options), None)?;

    println!("Analyzing...");

    let remote_master_oid = repo.refname_to_id("refs/remotes/origin/master")?;
    let remote_head = repo.find_annotated_commit(remote_master_oid)?;

    let (analysis, _preference) = repo.merge_analysis(&[&remote_head])?;

    if analysis.is_up_to_date() {
        println!("merge analysis: up to date");
        // pull done

        Ok(())
    } else if analysis.is_fast_forward() {
        println!("merge analysis: fast forward");

        let remote_master = repo.find_object(remote_master_oid, None)?;
        let mut opts = CheckoutBuilder::new();
        opts.safe();
        opts.recreate_missing(true);
        let _ = repo.checkout_tree(&remote_master, Some(&mut opts))?;

        let mut local_master = repo.find_reference("refs/heads/master")?;
        let _ = local_master.set_target(remote_master_oid, "fast forwarding to remote master")?;
        let mut head = repo.head()?;
        let _ = head.set_target(remote_master_oid, "fast forwarding to remote master")?;

        Ok(())
    } else if analysis.is_normal() {
        // TODO: resolve conflicts
        // TODO: merge

        Err(ErrorKind::NotImplemented.into())
    } else {
        println!("unsupported merge analysis result: {:?}", analysis);
        bail!("Unsupported merge analysis result")
    }
}

fn find_non_colliding_name(repo: &Repository, name: &str, postfix: Option<i32>) -> Result<String> {
    let new_name = match postfix {
        Some(i) => format!("{}-w{}", name, i),
        None => name.to_string()
    };
    let ref_name = format!("refs/heads/{}", new_name);

    match repo.find_reference(&ref_name) {
        Ok(_) => {
            let next_postfix = Some(postfix.unwrap_or(0) + 1);
            find_non_colliding_name(repo, name, next_postfix)
        }
        Err(_) => {
            Ok(new_name)
        }
    }
}

fn print_manual_merge_master(branch_name: &str) {
    println!("merge is not implemented yet!");
    println!("run manually:");
    println!("- git merge origin/master");
    println!("- git mergetool");
    println!("- git commit -a -m 'Merged changes'");
    println!("- git checkout -b {}", branch_name);
    println!("- git stash pop");
}

pub fn work_on(dir: &Path, name: &str) -> Result<()> {
    let mut repo = Repository::open(dir)?;

    if repo.state() == RepositoryState::Clean {
        let branch_name = get_current_branch(&repo)?;
        let re = Regex::new(format!(r"{}(-w\d+)?", name).as_str())?;

        if re.is_match(&branch_name) {
            println!("The current branch name ({}) already matches the specified one", branch_name);

            Ok(())
        } else {
            println!("Currently on a different branch: {}", branch_name);

            let safe_name = find_non_colliding_name(&repo, name, None)?;

            let stash_oid = stash_changes_if_any(&mut repo)?;
            checkout_master(&mut repo)?;

            match pull(&mut repo) {
                Ok(_) => {
                    create_branch(&mut repo, &safe_name)?;
                    checkout_branch(&mut repo, &safe_name)?;

                    match stash_oid {
                        Some(oid) => {
                            let _ = unstash_changes(&mut repo, oid)?;
                        }
                        None => {
                        }
                    };

                    Ok(())
                }
                Err(Error(ErrorKind::NotImplemented, _)) => {
                    print_manual_merge_master(&safe_name);
                    Ok(())
                }
                Err(other) => Err(other)
            }
        }
    } else {
        bail!("The repository's state is not clean (but {:?})", repo.state())
    }
}
