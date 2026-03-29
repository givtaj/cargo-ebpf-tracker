use std::env;
use std::os::unix::process::CommandExt;
use std::process::{self, Command};

const INTERACTIVE_PTY_ENV_NAME: &str = "EBPF_TRACKER_INTERACTIVE_PTY";

fn main() {
    let arg_count = env::var("EBPF_TRACKER_ARG_COUNT")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(0);

    if arg_count == 0 {
        process::exit(0);
    }

    let mut args = Vec::with_capacity(arg_count);
    for index in 1..=arg_count {
        let var_name = format!("EBPF_TRACKER_ARG_{index}");
        match env::var(&var_name) {
            Ok(value) => args.push(value),
            Err(_) => {
                eprintln!("missing environment variable: {var_name}");
                process::exit(127);
            }
        }
    }

    let program = args[0].clone();
    if interactive_pty_enabled() {
        let command = shell_quote_command(&args);
        let wrapper = format!("exec script -qefc {} /dev/null >&2", shell_quote(&command));
        let error = Command::new("/bin/sh").arg("-lc").arg(wrapper).exec();
        eprintln!("failed to exec interactive PTY wrapper for {program}: {error}");
        process::exit(126);
    }

    let error = Command::new(&program).args(&args[1..]).exec();
    eprintln!("failed to exec {program}: {error}");
    process::exit(match error.kind() {
        std::io::ErrorKind::NotFound => 127,
        _ => 126,
    });
}

fn interactive_pty_enabled() -> bool {
    matches!(env::var(INTERACTIVE_PTY_ENV_NAME).as_deref(), Ok("1"))
}

fn shell_quote_command(args: &[String]) -> String {
    args.iter()
        .map(|arg| shell_quote(arg))
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }

    format!("'{}'", value.replace('\'', r#"'"'"'"#))
}
