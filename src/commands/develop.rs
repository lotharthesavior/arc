use std::path::PathBuf;
use std::process::ExitStatus;
use std::{fs, io};
use tokio::io::{AsyncBufReadExt, BufReader, Lines};
use tokio::process::{ChildStderr, ChildStdout, Command};
use tokio::task::JoinHandle;
use tokio::try_join;
use tracing::{debug, error, info};

pub async fn run_development() -> io::Result<()> {
    info!("Starting development mode");

    let cargo_watch_task: JoinHandle<io::Result<()>> = tokio::spawn(run_cargo_watch());
    let bundle_task: JoinHandle<io::Result<()>> = tokio::spawn(run_vite_bundle());

    match try_join!(cargo_watch_task, bundle_task) {
        Ok(_) => {
            info!("Development environment running successfully");
            Ok(())
        }
        Err(e) => {
            error!("Development environment error: {:?}", e);
            Err(io::Error::other("Failed to run development tasks"))
        }
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
            error!("[cargo-watch] {}", line);
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
            error!("[vite] {}", line);
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
