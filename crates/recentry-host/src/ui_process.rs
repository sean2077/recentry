use std::{
    os::windows::process::CommandExt,
    path::PathBuf,
    process::{Child, Command},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use recentry_ipc::{PipeConnection, PipeServer, connect};
use recentry_protocol::{UiCommand, UiResponse};
use windows_sys::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

enum CoordinatorRequest {
    Command {
        command: UiCommand,
        reply: mpsc::Sender<Result<UiResponse, String>>,
    },
    Shutdown,
}

struct UiConnection {
    child: Child,
    pipe: PipeConnection,
}

#[derive(Clone)]
pub struct UiCoordinator {
    sender: mpsc::Sender<CoordinatorRequest>,
}

impl UiCoordinator {
    pub fn start(
        ui_executable: PathBuf,
        config_path: PathBuf,
        host_pipe: String,
    ) -> (Self, thread::JoinHandle<()>) {
        let (sender, receiver) = mpsc::channel();
        let worker =
            thread::spawn(move || run_coordinator(receiver, ui_executable, config_path, host_pipe));
        (Self { sender }, worker)
    }

    pub fn request(&self, command: UiCommand) -> Result<UiResponse, String> {
        let (reply, response) = mpsc::channel();
        self.sender
            .send(CoordinatorRequest::Command { command, reply })
            .map_err(|_| "UI coordinator stopped".to_owned())?;
        response
            .recv_timeout(Duration::from_secs(10))
            .map_err(|_| "UI response timed out".to_owned())?
    }

    pub fn shutdown(&self) {
        let _ = self.sender.send(CoordinatorRequest::Shutdown);
    }
}

fn run_coordinator(
    receiver: mpsc::Receiver<CoordinatorRequest>,
    ui_executable: PathBuf,
    config_path: PathBuf,
    host_pipe: String,
) {
    let mut connection: Option<UiConnection> = None;
    let mut generation = 0u32;
    while let Ok(request) = receiver.recv() {
        match request {
            CoordinatorRequest::Shutdown => {
                stop_ui(&mut connection);
                break;
            }
            CoordinatorRequest::Command { command, reply } => {
                if matches!(command, UiCommand::Quit) {
                    stop_ui(&mut connection);
                    let _ = reply.send(Ok(UiResponse::Quitting));
                    continue;
                }
                if child_exited(&mut connection) {
                    connection = None;
                }
                if connection.is_none() {
                    generation = generation.wrapping_add(1);
                    match start_ui(&ui_executable, &config_path, &host_pipe, generation) {
                        Ok(ui) => connection = Some(ui),
                        Err(error) => {
                            let _ = reply.send(Err(error));
                            continue;
                        }
                    }
                }
                grant_foreground(connection.as_ref().unwrap(), &command);
                let first = transact(connection.as_ref().unwrap(), &command);
                let result = match first {
                    Ok(response) => Ok(response),
                    Err(first_error) if should_restart(&command) => {
                        stop_ui(&mut connection);
                        generation = generation.wrapping_add(1);
                        match start_ui(&ui_executable, &config_path, &host_pipe, generation) {
                            Ok(ui) => {
                                connection = Some(ui);
                                grant_foreground(connection.as_ref().unwrap(), &command);
                                transact(connection.as_ref().unwrap(), &command).map_err(|error| {
                                    format!("{first_error}; restart transaction failed: {error}")
                                })
                            }
                            Err(error) => Err(format!("{first_error}; restart failed: {error}")),
                        }
                    }
                    Err(error) => Err(error),
                };
                let _ = reply.send(result);
            }
        }
    }
}

fn grant_foreground(connection: &UiConnection, command: &UiCommand) {
    if matches!(
        command,
        UiCommand::Show | UiCommand::Settings(_) | UiCommand::Diagnostics(_)
    ) {
        unsafe { AllowSetForegroundWindow(connection.child.id()) };
    }
}

fn should_restart(command: &UiCommand) -> bool {
    matches!(
        command,
        UiCommand::Show | UiCommand::Settings(_) | UiCommand::Diagnostics(_)
    )
}

fn child_exited(connection: &mut Option<UiConnection>) -> bool {
    connection
        .as_mut()
        .and_then(|ui| ui.child.try_wait().ok())
        .flatten()
        .is_some()
}

fn start_ui(
    executable: &PathBuf,
    config_path: &PathBuf,
    host_pipe: &str,
    generation: u32,
) -> Result<UiConnection, String> {
    if !executable.is_file() {
        return Err("recentry-ui.exe is missing beside the host".to_owned());
    }
    let pipe_name = format!(r"\\.\pipe\recentry-ui-{}-{generation}", std::process::id());
    let server = PipeServer::bind(&pipe_name).map_err(|error| error.to_string())?;
    let mut child = Command::new(executable)
        .arg("--pipe")
        .arg(&pipe_name)
        .arg("--host-pipe")
        .arg(host_pipe)
        .arg("--config")
        .arg(config_path)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|error| format!("failed to start recentry-ui.exe: {error}"))?;

    let (accepted, receiver) = mpsc::channel();
    let accept_pipe_name = pipe_name.clone();
    let acceptor = thread::spawn(move || {
        let result = server.accept().map_err(|error| error.to_string());
        let _ = accepted.send(result);
    });
    let pipe = match receiver.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(pipe)) => pipe,
        Ok(Err(error)) => {
            let _ = child.kill();
            let _ = child.wait();
            let _ = acceptor.join();
            return Err(error);
        }
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            let _ = connect(&accept_pipe_name, 100);
            let _ = acceptor.join();
            return Err("recentry-ui.exe did not connect within 5 seconds".to_owned());
        }
    };
    let _ = acceptor.join();
    Ok(UiConnection { child, pipe })
}

fn transact(connection: &UiConnection, command: &UiCommand) -> Result<UiResponse, String> {
    connection
        .pipe
        .send(command)
        .map_err(|error| error.to_string())?;
    connection.pipe.receive().map_err(|error| error.to_string())
}

fn stop_ui(connection: &mut Option<UiConnection>) {
    let Some(ui) = connection.as_mut() else {
        return;
    };
    let _ = transact(ui, &UiCommand::Quit);
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if matches!(ui.child.try_wait(), Ok(Some(_))) {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    let _ = ui.child.kill();
    let _ = ui.child.wait();
    *connection = None;
}
