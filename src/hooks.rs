use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Shell hook installation and management
pub struct HookManager {
    repo_path: PathBuf,
}

#[derive(Debug, Clone)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    PowerShell,
}

impl HookManager {
    pub fn new(repo_path: &Path) -> Self {
        Self {
            repo_path: repo_path.to_path_buf(),
        }
    }

    /// Install hooks for the current shell
    pub fn install_hooks(&self) -> Result<()> {
        let shell = self.detect_shell()?;

        match shell {
            ShellType::Bash => self.install_bash_hooks(),
            ShellType::Zsh => self.install_zsh_hooks(),
            ShellType::Fish => self.install_fish_hooks(),
            ShellType::PowerShell => self.install_powershell_hooks(),
        }
    }

    pub fn config_file(&self) -> PathBuf {
        self.repo_path.join(".fukura").join("hooks.toml")
    }

    /// Uninstall hooks
    pub fn uninstall_hooks(&self) -> Result<()> {
        let shell = self.detect_shell()?;

        match shell {
            ShellType::Bash => self.uninstall_bash_hooks(),
            ShellType::Zsh => self.uninstall_zsh_hooks(),
            ShellType::Fish => self.uninstall_fish_hooks(),
            ShellType::PowerShell => self.uninstall_powershell_hooks(),
        }
    }

    /// Check if hooks are installed
    pub fn are_hooks_installed(&self) -> Result<bool> {
        let shell = self.detect_shell()?;

        match shell {
            ShellType::Bash => self.check_bash_hooks(),
            ShellType::Zsh => self.check_zsh_hooks(),
            ShellType::Fish => self.check_fish_hooks(),
            ShellType::PowerShell => self.check_powershell_hooks(),
        }
    }

    fn detect_shell(&self) -> Result<ShellType> {
        let shell = std::env::var("SHELL")
            .or_else(|_| std::env::var("SHELL_NAME"))
            .context("Could not detect shell")?;

        if shell.contains("bash") {
            Ok(ShellType::Bash)
        } else if shell.contains("zsh") {
            Ok(ShellType::Zsh)
        } else if shell.contains("fish") {
            Ok(ShellType::Fish)
        } else if shell.contains("powershell") || shell.contains("pwsh") {
            Ok(ShellType::PowerShell)
        } else {
            Err(anyhow::anyhow!("Unsupported shell: {}", shell))
        }
    }

    fn install_bash_hooks(&self) -> Result<()> {
        let bashrc_path = self.get_bashrc_path()?;
        let hook_content = self.generate_bash_hook();

        if !self.is_hook_installed(&bashrc_path, "bash")? {
            self.append_to_file(&bashrc_path, &hook_content)?;
            println!(" Installed Fukura hooks for bash");
        } else {
            println!("  Fukura hooks already installed for bash");
        }

        Ok(())
    }

    fn install_zsh_hooks(&self) -> Result<()> {
        let zshrc_path = self.get_zshrc_path()?;
        let hook_content = self.generate_zsh_hook();

        if !self.is_hook_installed(&zshrc_path, "zsh")? {
            self.append_to_file(&zshrc_path, &hook_content)?;
            println!(" Installed Fukura hooks for zsh");
        } else {
            println!("  Fukura hooks already installed for zsh");
        }

        Ok(())
    }

    fn install_fish_hooks(&self) -> Result<()> {
        let fish_config_dir = self.get_fish_config_dir()?;
        let hook_file = fish_config_dir.join("fukura_hooks.fish");

        if !hook_file.exists() {
            let hook_content = self.generate_fish_hook();
            fs::write(&hook_file, hook_content)?;
            println!(" Installed Fukura hooks for fish");
        } else {
            println!("  Fukura hooks already installed for fish");
        }

        Ok(())
    }

    fn install_powershell_hooks(&self) -> Result<()> {
        let profile_path = self.get_powershell_profile_path()?;
        let hook_content = self.generate_powershell_hook();

        if !self.is_hook_installed(&profile_path, "fukura")? {
            self.append_to_file(&profile_path, &hook_content)?;
            println!(" Installed Fukura hooks for PowerShell");
        } else {
            println!("  Fukura hooks already installed for PowerShell");
        }

        Ok(())
    }

    fn uninstall_bash_hooks(&self) -> Result<()> {
        let bashrc_path = self.get_bashrc_path()?;
        self.remove_hook_from_file(&bashrc_path, "fukura")?;
        println!(" Uninstalled Fukura hooks for bash");
        Ok(())
    }

    fn uninstall_zsh_hooks(&self) -> Result<()> {
        let zshrc_path = self.get_zshrc_path()?;
        self.remove_hook_from_file(&zshrc_path, "fukura")?;
        println!(" Uninstalled Fukura hooks for zsh");
        Ok(())
    }

    fn uninstall_fish_hooks(&self) -> Result<()> {
        let fish_config_dir = self.get_fish_config_dir()?;
        let hook_file = fish_config_dir.join("fukura_hooks.fish");

        if hook_file.exists() {
            fs::remove_file(&hook_file)?;
            println!(" Uninstalled Fukura hooks for fish");
        }

        Ok(())
    }

    fn uninstall_powershell_hooks(&self) -> Result<()> {
        let profile_path = self.get_powershell_profile_path()?;
        self.remove_hook_from_file(&profile_path, "fukura")?;
        println!(" Uninstalled Fukura hooks for PowerShell");
        Ok(())
    }

    fn check_bash_hooks(&self) -> Result<bool> {
        let bashrc_path = self.get_bashrc_path()?;
        self.is_hook_installed(&bashrc_path, "bash")
    }

    fn check_zsh_hooks(&self) -> Result<bool> {
        let zshrc_path = self.get_zshrc_path()?;
        self.is_hook_installed(&zshrc_path, "zsh")
    }

    fn check_fish_hooks(&self) -> Result<bool> {
        let fish_config_dir = self.get_fish_config_dir()?;
        let hook_file = fish_config_dir.join("fukura_hooks.fish");
        Ok(hook_file.exists())
    }

    fn check_powershell_hooks(&self) -> Result<bool> {
        let profile_path = self.get_powershell_profile_path()?;
        self.is_hook_installed(&profile_path, "powershell")
    }

    // Helper methods for file paths

    fn get_bashrc_path(&self) -> Result<std::path::PathBuf> {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        Ok(std::path::PathBuf::from(home).join(".bashrc"))
    }

    fn get_zshrc_path(&self) -> Result<std::path::PathBuf> {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        Ok(std::path::PathBuf::from(home).join(".zshrc"))
    }

    fn get_fish_config_dir(&self) -> Result<std::path::PathBuf> {
        let config_dir = if cfg!(target_os = "windows") {
            std::env::var("APPDATA")?
        } else {
            std::env::var("XDG_CONFIG_HOME")
                .or_else(|_| std::env::var("HOME").map(|h| format!("{}/.config", h)))?
        };

        Ok(std::path::PathBuf::from(config_dir).join("fish"))
    }

    fn get_powershell_profile_path(&self) -> Result<std::path::PathBuf> {
        let output = Command::new("powershell")
            .args(["-Command", "$PROFILE"])
            .output()?;

        let profile_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(std::path::PathBuf::from(profile_path))
    }

    // Helper methods for file operations

    fn is_hook_installed(&self, file_path: &Path, hook_name: &str) -> Result<bool> {
        if !file_path.exists() {
            return Ok(false);
        }

        let content = fs::read_to_string(file_path)?;
        Ok(content.contains(&format!("# Fukura hooks - {}", hook_name)))
    }

    fn append_to_file(&self, file_path: &Path, content: &str) -> Result<()> {
        if file_path.exists() {
            let mut existing_content = fs::read_to_string(file_path)?;
            existing_content.push('\n');
            existing_content.push_str(content);
            fs::write(file_path, existing_content)?;
        } else {
            fs::write(file_path, content)?;
        }
        Ok(())
    }

    fn remove_hook_from_file(&self, file_path: &Path, hook_name: &str) -> Result<()> {
        if !file_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(file_path)?;
        let lines: Vec<&str> = content.lines().collect();
        let mut new_lines = Vec::new();
        let mut in_hook_section = false;

        for line in lines {
            if line.contains(&format!("# Fukura hooks - {}", hook_name)) {
                in_hook_section = true;
                continue;
            }

            if in_hook_section && line.trim().is_empty() && !line.contains("# Fukura hooks") {
                in_hook_section = false;
            }

            if !in_hook_section {
                new_lines.push(line);
            }
        }

        fs::write(file_path, new_lines.join("\n"))?;
        Ok(())
    }

    // Hook content generators

    fn generate_bash_hook(&self) -> String {
        format!(
            r#"
# Fukura hooks - bash (World-class IPC via Unix Domain Socket)
_fukura_socket_path="{socket_path}"
_fukura_last_command=""

_fukura_record_command() {{
    local exit_code=$?
    local command="$_fukura_last_command"
    local working_dir="$PWD"
    local session_id=$(echo "$PWD" | md5sum 2>/dev/null | cut -d' ' -f1 || echo "default")
    
    if [ -n "$command" ] && [ -S "$_fukura_socket_path" ]; then
        echo "$session_id|$command|$exit_code|$working_dir" | nc -U -w 1 "$_fukura_socket_path" 2>/dev/null || true
    fi
}}

_fukura_preexec() {{
    _fukura_last_command="$BASH_COMMAND"
}}

# Hook into command execution
trap '_fukura_preexec' DEBUG

# Hook into prompt
if [[ -z "$PROMPT_COMMAND" ]]; then
    PROMPT_COMMAND="_fukura_record_command"
else
    PROMPT_COMMAND="${{PROMPT_COMMAND}}; _fukura_record_command"
fi
"#,
            socket_path = self.repo_path.join(".fukura").join("daemon.sock").display()
        )
    }

    fn generate_zsh_hook(&self) -> String {
        format!(
            r#"
# Fukura hooks - zsh (World-class IPC via Unix Domain Socket)
_fukura_socket_path="{socket_path}"

_fukura_record_command() {{
    local exit_code=$?
    local command="$1"
    local working_dir="$PWD"
    local session_id="$(echo "$PWD" | md5sum 2>/dev/null | cut -d' ' -f1 || echo "default")"
    
    # Send to daemon via Unix socket (fast & secure)
    if [ -S "$_fukura_socket_path" ]; then
        echo "$session_id|$command|$exit_code|$working_dir" | nc -U -w 1 "$_fukura_socket_path" 2>/dev/null || true
    fi
}}

# Hook into command execution
preexec_functions+=(_fukura_preexec_hook)
precmd_functions+=(_fukura_precmd_hook)

_fukura_preexec_hook() {{
    _fukura_last_command="$1"
}}

_fukura_precmd_hook() {{
    local exit_code=$?
    if [ -n "$_fukura_last_command" ]; then
        _fukura_record_command "$_fukura_last_command"
    fi
}}
"#,
            socket_path = self.repo_path.join(".fukura").join("daemon.sock").display()
        )
    }

    fn generate_fish_hook(&self) -> String {
        r#"
# Fukura hooks - fish
function _fukura_record_command --on-event fish_prompt
    set -l exit_code $status
    set -l command (history | head -n1)
    set -l working_dir (pwd)
    
    # Record command with exit code
    fukura daemon record-command (pwd | tr '/' '_') "$command" "$exit_code" "$working_dir" 2>/dev/null || true
end

function _fukura_record_error --on-event fish_postexec
    set -l exit_code $status
    if [ $exit_code -ne 0 ]
        # Record the command as an error
        fukura daemon record-error (pwd | tr '/' '_') "Command failed with exit code $exit_code" "fish" 2>/dev/null || true
    end
end
"#.to_string()
    }

    fn generate_powershell_hook(&self) -> String {
        r#"
# Fukura hooks - PowerShell
function _fukura_record_command {{
    param($command, $exitCode, $workingDir)
    
    # Record command with exit code
    fukura daemon record-command (Get-Location | ForEach-Object {{ $_.Path -replace '\\\\', '_' -replace ':', '_' }}) "$command" "$exitCode" "$workingDir" 2>$null
}}

# Override Invoke-Expression to capture commands
$originalInvokeExpression = Get-Command Invoke-Expression
function Invoke-Expression {{
    param($command)
    
    try {{
        & $originalInvokeExpression $command
        _fukura_record_command $command $LASTEXITCODE (Get-Location).Path
    }}
    catch {{
        _fukura_record_command $command 1 (Get-Location).Path
        throw
    }}
}}
"#.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_hook_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = HookManager::new(temp_dir.path());
        assert!(manager.repo_path.exists());
    }

    #[test]
    fn test_shell_detection() {
        std::env::set_var("SHELL", "/bin/bash");
        let temp_dir = TempDir::new().unwrap();
        let manager = HookManager::new(temp_dir.path());

        let shell = manager.detect_shell().unwrap();
        assert!(matches!(shell, ShellType::Bash));
    }

    #[test]
    fn test_hook_content_generation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = HookManager::new(temp_dir.path());

        let bash_hook = manager.generate_bash_hook();
        assert!(bash_hook.contains("fukura daemon"));
        assert!(bash_hook.contains("bash"));

        let zsh_hook = manager.generate_zsh_hook();
        assert!(zsh_hook.contains("fukura daemon"));
        assert!(zsh_hook.contains("zsh"));
    }
}
