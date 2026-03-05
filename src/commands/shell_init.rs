use clap::Command;
use clap_complete::{Shell, generate};
use std::io::Write;

const BASH_WRAPPER: &str = r#"
# repoman shell wrapper — intercepts `repoman open` to cd into the directory
repoman() {
    if [ "$1" = "open" ]; then
        local dir
        dir="$(command repoman open "${@:2}")" || return $?
        cd "$dir" || return $?
    else
        command repoman "$@"
    fi
}
"#;

const ZSH_WRAPPER: &str = r#"
# repoman shell wrapper — intercepts `repoman open` to cd into the directory
repoman() {
    if [[ "$1" == "open" ]]; then
        local dir
        dir="$(command repoman open "${@:2}")" || return $?
        cd "$dir" || return $?
    else
        command repoman "$@"
    fi
}
"#;

const FISH_WRAPPER: &str = r#"
# repoman shell wrapper — intercepts `repoman open` to cd into the directory
function repoman --wraps=repoman
    if test "$argv[1]" = "open"
        set -l dir (command repoman open $argv[2..])
        or return $status
        cd $dir
        or return $status
    else
        command repoman $argv
    end
end
"#;

pub fn handle_shell_init(shell: Shell, cmd: &mut Command) {
    // Generate completions into a buffer
    let mut buf = Vec::new();
    generate(shell, cmd, "repoman", &mut buf);

    // Write completions
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(&buf);

    // Append shell wrapper function
    let wrapper = match shell {
        Shell::Bash => BASH_WRAPPER,
        Shell::Zsh => ZSH_WRAPPER,
        Shell::Fish => FISH_WRAPPER,
        _ => {
            // Elvish, PowerShell — completions only
            let _ = writeln!(
                out,
                "\n# Note: shell wrapper for `repoman open` is not yet supported for this shell."
            );
            let _ = writeln!(
                out,
                "# Completions have been generated. Use `cd $(repoman open <target>)` manually."
            );
            return;
        }
    };

    let _ = out.write_all(wrapper.as_bytes());
}
