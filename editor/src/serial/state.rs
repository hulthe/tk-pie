use std::{collections::VecDeque, path::PathBuf};

use tk_pie::serial_proto::owned::{ChangeLayer, DeviceMsg, LogRecord};
use tokio::sync::{
    mpsc::{self, Receiver},
    oneshot::{self, error::TryRecvError},
};

pub struct SerialState {
    pub scan_task: Option<oneshot::Receiver<Result<Option<PathBuf>, String>>>,
    pub dev: Result<Option<PathBuf>, String>,
    pub reader: Option<Receiver<DeviceMsg>>,
    pub logs: VecDeque<LogRecord>,
    pub active_layer: Option<(u16, u16)>,
}

impl Default for SerialState {
    fn default() -> Self {
        Self {
            scan_task: Default::default(),
            dev: Ok(None),
            reader: Default::default(),
            logs: Default::default(),
            active_layer: None,
        }
    }
}

impl SerialState {
    // TODO: Name?
    pub fn check_serial(&mut self) {
        let Self {
            scan_task: scan_serial_task,
            dev: serial_devs,
            reader: serial_reader,
            logs: serial_logs,
            active_layer,
        } = self;

        while let Some(rx) = scan_serial_task {
            match rx.try_recv() {
                Ok(r) => {
                    *serial_devs = r;
                    *scan_serial_task = None;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Closed) => {
                    *scan_serial_task = None;
                    break;
                }
            }
        }

        while let Some(rx) = serial_reader {
            match rx.try_recv() {
                Ok(DeviceMsg::Log(record)) => {
                    serial_logs.push_back(record);
                    if serial_logs.len() > 10 {
                        serial_logs.pop_front();
                    }
                }
                Ok(DeviceMsg::ChangeLayer(ChangeLayer { x, y })) => {
                    *active_layer = Some((x, y));
                }
                Ok(_) => {}
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    *serial_reader = None;
                    break;
                }
            }
        }
    }
}
