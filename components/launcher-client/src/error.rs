use habitat_launcher_protocol as protocol;
use ipc_channel::ipc::IpcError;
use std::{fmt,
          io};
use thiserror::Error;

/// Errors that occur when attempting to estabish an IPC channel to the Habitat Launcher
#[derive(Debug, Error)]
pub enum ConnectError {
    #[error("Failed to establish IPC connection to the launcher")]
    LauncherUnreachable(#[source] io::Error),
    #[error("Failed to start IPC server to listen for responses from launcher")]
    IPCServerStartup(#[source] io::Error),
    #[error("Failed to accept incoming IPC connection from the launcher")]
    IPCIncomingConnection(#[source] ipc_channel::Error),
    #[error("Failed to send registration IPC command to the launcher")]
    LauncherRegisterSend(#[source] SendError),
    #[error("Failed to receive registration IPC command response from the launcher")]
    LauncherRegisterReceive(#[source] IPCReadError),
}

/// Errors that occur when remotely executing a command on the Habitat Launcher
#[derive(Debug, Error)]
pub enum IPCCommandError {
    #[error("Failed to send '{0}' command to launcher")]
    Send(&'static str, #[source] SendError),
    #[error("Failed to receive '{0}' command response from launcher")]
    Receive(&'static str, #[source] ReceiveError),
}

/// Errors that occur when trying to remotely executing a command on the Habitat Launcher
#[derive(Debug, Error)]
pub enum TryIPCCommandError {
    #[error("Failed to send '{0}' command to launcher")]
    Send(&'static str, #[source] SendError),
    #[error("Failed to try receiving '{0}' command response from launcher")]
    TryReceive(&'static str, #[source] TryReceiveError),
}

/// Errors that occur when attempting to read an IPC response from the Habitat Launcher
#[derive(Debug, Error)]
pub enum IPCReadError {
    #[error("Failed to deserialize launcher protocol message: {0}")]
    ProtocolDeserialize(protocol::Error),
    #[error("Received an unexpected launcher protocol message payload: {0}")]
    PayloadDeserialize(protocol::Error),
    #[error("Launcher command execution failed: {0}")]
    LauncherCommand(protocol::NetErr),
}

///  Errors that occur when attempting to send a command to the Habitat Launcher via IPC
#[derive(Debug, Error)]
pub enum SendError {
    #[error("Failed to serialize launcher protocol message: {0}")]
    ProtocolSerialize(protocol::Error),
    #[error("Failed to serialize launcher protocol message payload: {0}")]
    PayloadSerialize(protocol::Error),
    #[error("Failed to send command to launcher")]
    IPCSend(#[source] ipc_channel::Error),
}

/// Errors that occur when attempting to blocking receive command responses from the Habitat
/// Launcher via IPC
#[derive(Debug, Error)]
pub enum ReceiveError {
    #[error("Failed to read launcher command response")]
    IPCRead(#[from] IPCReadError),
    #[error("Failed to receive IPC command response from launcher")]
    IPCReceive(#[from] IPCError),
}

/// Errors that occur when attempting to non-blocking receive command responses from the Habitat
/// Launcher via IPC
#[derive(Debug, Error)]
pub enum TryReceiveError {
    #[error("Failed to try reading launcher command response")]
    IPCRead(#[from] IPCReadError),
    #[error("Failed to try receiving IPC command response from launcher")]
    IPCReceive(#[from] IPCError),
    #[error("Timed out trying to receive IPC command response from launcher")]
    Timeout,
}

// TODO: Remove this wrapper type once we upgrade ipc-channel to 0.16+
// This is a wrapper type around the ipc_channel::ipc::IpcError that is
// required because it does not implement std::error::Error prior to
// version 0.16+. We need an older version of the crate to ensure that
// habitat works for some of our customers using Windows 7 until Jan 2024.
#[derive(Debug)]
pub struct IPCError(pub ipc_channel::ipc::IpcError);

impl From<ipc_channel::ipc::IpcError> for IPCError {
    fn from(value: ipc_channel::ipc::IpcError) -> Self { IPCError(value) }
}

impl fmt::Display for IPCError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            IpcError::Bincode(ref err) => write!(fmt, "bincode error: {}", err),
            IpcError::Io(ref err) => write!(fmt, "io error: {}", err),
            IpcError::Disconnected => write!(fmt, "disconnected"),
        }
    }
}

impl std::error::Error for IPCError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self.0 {
            IpcError::Bincode(ref err) => Some(err),
            IpcError::Io(ref err) => Some(err),
            IpcError::Disconnected => None,
        }
    }
}
