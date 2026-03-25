use std::env;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<i32, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut command = ebpf_tracker_viewer::build_node_command(&args)?;
    let mut child = command
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("failed to run viewer: {err}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to capture viewer stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "failed to capture viewer stderr".to_string())?;

    let out_handle = thread::spawn(move || forward_stdout(stdout));
    let err_handle = thread::spawn(move || forward_stderr(stderr));

    let status = child
        .wait()
        .map_err(|err| format!("failed waiting for viewer: {err}"))?;

    let out_result = out_handle
        .join()
        .map_err(|_| "viewer stdout forwarding thread panicked".to_string())?;
    out_result.map_err(|err| format!("viewer stdout forwarding failed: {err}"))?;

    let err_result = err_handle
        .join()
        .map_err(|_| "viewer stderr forwarding thread panicked".to_string())?;
    err_result.map_err(|err| format!("viewer stderr forwarding failed: {err}"))?;

    Ok(exit_code(status))
}

fn forward_stdout<R: Read>(mut reader: R) -> io::Result<()> {
    let mut stdout = io::stdout();
    let mut buffer = [0u8; 16 * 1024];

    loop {
        let read_bytes = reader.read(&mut buffer)?;
        if read_bytes == 0 {
            break;
        }
        stdout.write_all(&buffer[..read_bytes])?;
        stdout.flush()?;
    }

    Ok(())
}

fn forward_stderr<R: Read>(reader: R) -> io::Result<()> {
    let mut reader = BufReader::new(reader);
    let mut stderr = io::stderr();
    let mut opened = false;
    let mut line = Vec::new();

    loop {
        line.clear();
        let read_bytes = reader.read_until(b'\n', &mut line)?;
        if read_bytes == 0 {
            break;
        }

        let text = String::from_utf8_lossy(&line);
        if !opened {
            if let Some(url) = parse_dashboard_url(&text) {
                if let Err(err) = try_open_browser(url) {
                    writeln!(stderr, "viewer ready at {url} ({err})")?;
                }
                opened = true;
            }
        }

        stderr.write_all(&line)?;
        stderr.flush()?;
    }

    Ok(())
}

fn parse_dashboard_url(line: &str) -> Option<&str> {
    line.trim()
        .strip_prefix("live trace viewer on ")
        .map(str::trim)
}

fn try_open_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(url);
        command
    };

    #[cfg(target_os = "linux")]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(url);
        command
    };

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("cmd");
        command.arg("/C").arg("start").arg("").arg(url);
        command
    };

    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| format!("failed to launch browser for {url}: {err}"))?;

    Ok(())
}

fn exit_code(status: ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return 128 + signal;
        }
    }

    1
}
