use std::path::PathBuf;
use std::process::ExitStatus;
use std::{fs, io};
use tokio::io::{AsyncBufReadExt, BufReader, Lines};
use tokio::process::{ChildStderr, ChildStdout, Command};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

pub async fn run_development() -> io::Result<()> {
    info!("Starting development mode");

    let mut cargo_watch_task: JoinHandle<io::Result<()>> = tokio::spawn(run_cargo_watch());
    let mut bundle_task: JoinHandle<io::Result<()>> = tokio::spawn(run_vite_bundle());

    tokio::select! {
        result = &mut cargo_watch_task => {
            bundle_task.abort();
            handle_development_task_result("Cargo Watch", result)
        }
        result = &mut bundle_task => {
            cargo_watch_task.abort();
            handle_development_task_result("Vite", result)
        }
    }
}

fn handle_development_task_result(
    name: &str,
    result: Result<io::Result<()>, tokio::task::JoinError>,
) -> io::Result<()> {
    match result {
        Ok(Ok(())) => {
            info!("{name} process exited successfully");
            Ok(())
        }
        Ok(Err(err)) => {
            error!("{name} process exited with error: {err}");
            Err(err)
        }
        Err(err) => {
            error!("{name} task failed: {err}");
            Err(io::Error::other(format!("{name} task failed: {err}")))
        }
    }
}

fn log_child_stderr(prefix: &str, line: &str) {
    let trimmed = line.trim_start();
    let is_error = trimmed.starts_with("Error:") || trimmed.starts_with("error:");

    if is_error {
        error!("[{prefix}] {line}");
    } else {
        warn!("[{prefix}] {line}");
    }
}

async fn run_cargo_watch() -> io::Result<()> {
    let mut cargo_watch_process = Command::new("cargo")
        .arg("watch")
        .arg("-x")
        .arg("run serve")
        .arg("-i")
        .arg("database/*")
        .arg("-i")
        .arg("dist/*")
        .arg("-i")
        .arg("node_modules/*")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start Cargo Watch");

    let stdout = cargo_watch_process
        .stdout
        .take()
        .expect("Failed to capture stdout");
    let stderr = cargo_watch_process
        .stderr
        .take()
        .expect("Failed to capture stderr");

    let stdout_task = tokio::spawn(async move {
        let mut reader: Lines<BufReader<ChildStdout>> = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            debug!("[cargo-watch] {}", line);
        }
    });

    let stderr_task = tokio::spawn(async move {
        let mut reader: Lines<BufReader<ChildStderr>> = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            log_child_stderr("cargo-watch", &line);
        }
    });

    let status = cargo_watch_process
        .wait()
        .await
        .expect("Cargo Watch process wasn't running");

    stdout_task.await.expect("Failed to handle stdout");
    stderr_task.await.expect("Failed to handle stderr");

    if !status.success() {
        return Err(io::Error::other(format!(
            "Cargo Watch process exited with status: {:?}",
            status
        )));
    }

    Ok(())
}

async fn run_vite_bundle() -> io::Result<()> {
    if !fs::exists(PathBuf::from("node_modules")).unwrap() {
        let mut npm_install_process = Command::new("npm")
            .arg("install")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to install nodejs dependencies!");

        let status: ExitStatus = npm_install_process
            .wait()
            .await
            .expect("Npm Install wasn't running");

        if !status.success() {
            return Err(io::Error::other(format!(
                "Npm Install process exited with status: {:?}",
                status
            )));
        }
    }

    let mut vite_process = Command::new("npx")
        .arg("vite")
        .arg("build")
        .arg("--watch")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start Vite dev server");

    let stdout = vite_process
        .stdout
        .take()
        .expect("Failed to capture stdout");
    let stderr = vite_process
        .stderr
        .take()
        .expect("Failed to capture stderr");

    let stdout_task = tokio::spawn(async move {
        let mut reader: Lines<BufReader<ChildStdout>> = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            debug!("[vite] {}", line);
        }
    });

    let stderr_task: JoinHandle<()> = tokio::spawn(async move {
        let mut reader: Lines<BufReader<ChildStderr>> = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            log_child_stderr("vite", &line);
        }
    });

    let status: ExitStatus = vite_process
        .wait()
        .await
        .expect("Vite process wasn't running");

    stdout_task.await.expect("Failed to handle stdout");
    stderr_task.await.expect("Failed to handle stderr");

    if !status.success() {
        return Err(io::Error::other(format!(
            "Vite process exited with status: {:?}",
            status
        )));
    }

    Ok(())
}
