use std::{future::pending, path::PathBuf, time::Duration};

use egui::Context;
use eyre::{bail, Context as EyreContext};
use msgpck::{MsgPack, MsgUnpack, UnpackErr};
use tangentbord1::serial_proto::owned::{DeviceMsg, HostMsg};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
    select,
    sync::mpsc::{self, Receiver, Sender},
    time::{sleep, Instant},
};

const MAX_MESSAGE_SIZE: usize = 16 * 1024;
const MESSAGE_TIMEOUT: Duration = Duration::from_millis(30);

pub fn connect_to_serial(dev: PathBuf, ctx: Context) -> Receiver<DeviceMsg> {
    let (tx, rx) = mpsc::channel(12);
    tokio::spawn(async {
        if let Err(e) = read_serial(dev, tx, ctx).await {
            log::error!("serial read task exited with error: {e:#?}");
        }
    });
    rx
}

async fn read_serial(dev: PathBuf, tx: Sender<DeviceMsg>, ctx: Context) -> eyre::Result<()> {
    log::debug!("configuring keyboard serial device");
    let out = Command::new("stty")
        .arg("-F")
        .arg(&dev)
        .args(["115200", "raw", "-clocal", "-echo"])
        .output()
        .await
        .wrap_err("failed to configure serial device, couldn't execute stty")?;

    if !out.status.success() {
        bail!("failed to configure serial device");
    }

    log::debug!("opening keyboard serial device");
    let mut file = File::options()
        .create(false)
        .read(true)
        .append(true)
        .open(dev)
        .await?;

    log::debug!("requesting keyboard layers");
    for p in HostMsg::GetLayers.pack() {
        log::debug!("packing {:x?}", p);
        file.write_all(p.as_bytes()).await?;
    }

    let mut buf = Vec::with_capacity(MAX_MESSAGE_SIZE);
    let mut last_read = Instant::now();

    loop {
        // if buffer is not empty, this future will sleep until the pending message times out
        let timeout = async {
            if buf.is_empty() {
                pending().await
            } else {
                let timeout_at = last_read + MESSAGE_TIMEOUT;
                sleep(Instant::now() - timeout_at).await;
            }
        };

        // try to read some bytes from the file
        let mut tmp = [0u8; 1024];
        let n = select! {
            n = file.read(&mut tmp) => n?,

            // need to continuously poll read if nothing is happening
            _ = sleep(Duration::from_millis(20)) => continue,

            _ = timeout => {
                log::warn!("message timeout, clearing buffer");
                buf.clear();
                continue;
            }
        };

        // exit on eof
        if n == 0 {
            break;
        }

        last_read = Instant::now();
        buf.extend_from_slice(&tmp[..n]);

        // make sure we're not just reading garbage forever
        if buf.len() > MAX_MESSAGE_SIZE {
            log::warn!("max message size exceeded");
            buf.clear();
            continue;
        }

        // try to parse messages from the read bytes
        loop {
            let reader = &mut &buf[..];
            let record = match DeviceMsg::unpack(reader) {
                Ok(r) => r,

                // we probably have not gotten the entire message yet, go back to reading bytes.
                // if the message is corrupted, we will eventually hit MESSAGE_TIMEOUT or
                // MAX_MESSAGE_SIZE.
                Err(UnpackErr::UnexpectedEof) => break,

                // on any other error, the message is corrupt. clear the buffer.
                Err(e) => {
                    log::warn!("corrupt message: {e:?}");
                    buf.clear();
                    break;
                }
            };

            // remove the decoded bytes from buf
            if reader.is_empty() {
                buf.clear();
            } else {
                let bytes_read = buf.len() - reader.len();
                buf.rotate_left(bytes_read);
                buf.truncate(buf.len() - bytes_read);
            }

            if tx.send(record).await.is_err() {
                log::info!("channel closed, closing serial thingy");
                return Ok(());
            }

            // if there are no more bytes, stop trying to decode messages.
            if buf.is_empty() {
                break;
            }
        }

        ctx.request_repaint();
    }

    Ok(())
}
