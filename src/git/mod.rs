use std::process::Command;
use crate::proxy::ProxyConfig;

#[derive(Debug)]
pub struct GitResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

impl GitResult {
    fn from_output(output: std::process::Output) -> Self {
        GitResult {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        }
    }
}

/// Create a `Command` for git with optional proxy environment variables.
pub fn build_git_command(proxy: Option<&ProxyConfig>) -> Command {
    let mut cmd = Command::new("git");
    if let Some(config) = proxy {
        let url = config.to_url();
        match config.proxy_type.as_str() {
            "socks5" => {
                cmd.env("ALL_PROXY", &url);
            }
            _ => {
                cmd.env("HTTP_PROXY", &url);
                cmd.env("HTTPS_PROXY", &url);
            }
        }
    }
    cmd
}

/// Check if git is available on the system.
pub fn is_git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Clone a git repository to a local path.
pub fn clone(url: &str, local_path: &str, proxy: Option<&ProxyConfig>) -> Result<GitResult, String> {
    let output = build_git_command(proxy)
        .args(["clone", url, local_path])
        .output()
        .map_err(|e| format!("Failed to execute git clone: {}", e))?;
    Ok(GitResult::from_output(output))
}

/// Pull latest changes using fast-forward only.
pub fn pull(repo_path: &str, proxy: Option<&ProxyConfig>) -> Result<GitResult, String> {
    let output = build_git_command(proxy)
        .args(["pull", "--ff-only"])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to execute git pull: {}", e))?;
    Ok(GitResult::from_output(output))
}

/// Fetch from remote.
pub fn fetch(repo_path: &str, proxy: Option<&ProxyConfig>) -> Result<GitResult, String> {
    let output = build_git_command(proxy)
        .arg("fetch")
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to execute git fetch: {}", e))?;
    Ok(GitResult::from_output(output))
}

/// Check if the remote has changes not yet pulled locally.
/// Fetches first, then compares HEAD with upstream.
pub fn has_remote_changes(repo_path: &str, proxy: Option<&ProxyConfig>) -> Result<bool, String> {
    // Fetch first to get latest remote refs
    fetch(repo_path, proxy)?;

    let output = Command::new("git")
        .args(["rev-list", "HEAD..@{u}", "--count"])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to check remote changes: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("Failed to check remote changes: {}", stderr));
    }

    let count_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let count: u64 = count_str.parse().unwrap_or(0);
    Ok(count > 0)
}

/// Check if the local repo has uncommitted or unpushed changes.
pub fn has_local_changes(repo_path: &str) -> Result<bool, String> {
    // Check for uncommitted changes
    let status_output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to check local changes: {}", e))?;

    if !status_output.status.success() {
        let stderr = String::from_utf8_lossy(&status_output.stderr).trim().to_string();
        return Err(format!("Failed to check git status: {}", stderr));
    }

    let status_str = String::from_utf8_lossy(&status_output.stdout).trim().to_string();
    if !status_str.is_empty() {
        return Ok(true);
    }

    // Check for unpushed commits
    let rev_output = Command::new("git")
        .args(["rev-list", "@{u}..HEAD", "--count"])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to check unpushed commits: {}", e))?;

    if !rev_output.status.success() {
        // If upstream is not configured, there may be local-only commits
        // but we can't compare; treat as no unpushed changes
        return Ok(false);
    }

    let count_str = String::from_utf8_lossy(&rev_output.stdout).trim().to_string();
    let count: u64 = count_str.parse().unwrap_or(0);
    Ok(count > 0)
}

/// Stage all changes, commit with the given message, and push.
/// Handles "nothing to commit" gracefully by returning success.
pub fn commit_and_push(repo_path: &str, message: &str, proxy: Option<&ProxyConfig>) -> Result<GitResult, String> {
    // git add -A
    let add_output = build_git_command(proxy)
        .args(["add", "-A"])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to execute git add: {}", e))?;

    if !add_output.status.success() {
        return Ok(GitResult::from_output(add_output));
    }

    // git commit -m <message>
    let commit_output = build_git_command(proxy)
        .args(["commit", "-m", message])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to execute git commit: {}", e))?;

    // Handle "nothing to commit" gracefully
    if !commit_output.status.success() {
        let stdout = String::from_utf8_lossy(&commit_output.stdout);
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        if stdout.contains("nothing to commit") || stderr.contains("nothing to commit") {
            return Ok(GitResult {
                success: true,
                stdout: "nothing to commit".to_string(),
                stderr: String::new(),
            });
        }
        return Ok(GitResult::from_output(commit_output));
    }

    // git push
    let push_output = build_git_command(proxy)
        .arg("push")
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to execute git push: {}", e))?;

    Ok(GitResult::from_output(push_output))
}

/// Extract the repository name from a git URL.
/// Handles HTTPS URLs, SSH URLs, and strips `.git` suffix and trailing slashes.
pub fn extract_repo_name(url: &str) -> String {
    let url = url.trim_end_matches('/');

    // Get the last path segment
    let name = if let Some(pos) = url.rfind('/') {
        &url[pos + 1..]
    } else if let Some(pos) = url.rfind(':') {
        // SSH format: git@github.com:user/repo.git
        let after_colon = &url[pos + 1..];
        if let Some(slash_pos) = after_colon.rfind('/') {
            &after_colon[slash_pos + 1..]
        } else {
            after_colon
        }
    } else {
        url
    };

    // Strip .git suffix
    name.strip_suffix(".git").unwrap_or(name).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_git_available() {
        assert!(is_git_available());
    }

    #[test]
    fn test_extract_repo_name_https() {
        assert_eq!(
            extract_repo_name("https://github.com/user/my-repo.git"),
            "my-repo"
        );
    }

    #[test]
    fn test_extract_repo_name_https_no_git_suffix() {
        assert_eq!(
            extract_repo_name("https://github.com/user/my-repo"),
            "my-repo"
        );
    }

    #[test]
    fn test_extract_repo_name_ssh() {
        assert_eq!(
            extract_repo_name("git@github.com:user/my-repo.git"),
            "my-repo"
        );
    }

    #[test]
    fn test_extract_repo_name_trailing_slash() {
        assert_eq!(
            extract_repo_name("https://github.com/user/my-repo/"),
            "my-repo"
        );
    }

    #[test]
    fn test_build_git_command_no_proxy() {
        let cmd = build_git_command(None);
        assert!(format!("{:?}", cmd).contains("\"git\""));
    }

    #[test]
    fn test_build_git_command_with_http_proxy() {
        use crate::proxy::ProxyConfig;
        let config = ProxyConfig {
            enabled: true,
            proxy_type: "http".to_string(),
            host: "127.0.0.1".to_string(),
            port: "7890".to_string(),
            username: None,
            password: None,
        };
        let cmd = build_git_command(Some(&config));
        assert!(format!("{:?}", cmd).contains("\"git\""));
    }

    #[test]
    fn test_build_git_command_with_socks5_proxy() {
        use crate::proxy::ProxyConfig;
        let config = ProxyConfig {
            enabled: true,
            proxy_type: "socks5".to_string(),
            host: "127.0.0.1".to_string(),
            port: "1080".to_string(),
            username: None,
            password: None,
        };
        let cmd = build_git_command(Some(&config));
        assert!(format!("{:?}", cmd).contains("\"git\""));
    }
}
