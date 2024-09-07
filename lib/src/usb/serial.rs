use crate::{
    logger::{LogOutput, TimestampedRecord},
    serial_proto::borrowed::{DeviceMsg, LogRecord},
    serial_proto::owned::HostMsg,
    util::{DisplayPack, CS},
};

use super::MAX_PACKET_SIZE;
use core::future::pending;
use embassy_executor::Spawner;
use embassy_rp::{peripherals::USB, usb::Driver};
use embassy_sync::{channel::Channel, pipe::Pipe};
use embassy_time::{Duration, Timer};
use embassy_usb::{
    class::cdc_acm::{self, CdcAcmClass},
    Builder,
};
use embassy_usb_driver::EndpointError;
use futures::{select_biased, FutureExt};
use msgpck::{MsgPack, MsgUnpack, PackErr, UnpackErr};
use static_cell::StaticCell;

pub const BUFFER_SIZE: usize = 16 * 1024;
static OUT: Pipe<CS, BUFFER_SIZE> = Pipe::new();
pub static IN: Channel<CS, HostMsg, 16> = Channel::new(); // TODO: read from this guy

#[derive(Clone)]
pub struct UsbSerial;

pub async fn setup(usb_builder: &mut Builder<'static, Driver<'static, USB>>) -> &'static UsbSerial {
    let spawner = Spawner::for_current_executor().await;

    static STATE: StaticCell<cdc_acm::State<'static>> = StaticCell::new();
    let state = STATE.init(cdc_acm::State::new());

    let class = CdcAcmClass::new(usb_builder, state, MAX_PACKET_SIZE as u16);

    spawner.must_spawn(serial_task(class));

    static USB_SERIAL: UsbSerial = UsbSerial;
    &USB_SERIAL
}

async fn wait_connection(class: &mut CdcAcmClass<'static, Driver<'static, USB>>) {
    class.wait_connection().await;
    OUT.clear();
}

#[embassy_executor::task]
async fn serial_task(mut class: CdcAcmClass<'static, Driver<'static, USB>>) {
    let mut write_buf = [0u8; MAX_PACKET_SIZE as usize];
    let mut read_buf = [0u8; 1024];
    let mut message_parser = MessageParser::new(&mut read_buf);

    class.wait_connection().await;
    loop {
        select_biased! {
            n = OUT.read(&mut write_buf).fuse() => {
                let write = async {
                    class.write_packet(&write_buf[..n]).await?;

                    // if we send a packet containing exactly MAX_PACKET_SIZE bytes, we need to send another
                    // packet to "flush" the buffer.
                    if OUT.is_empty() && n == usize::from(MAX_PACKET_SIZE) {
                        class.write_packet(&[]).await?;
                    }

                    Ok(())
                };

                match write.await {
                    Ok(()) => {}
                    Err(EndpointError::Disabled) => wait_connection(&mut class).await,
                    Err(EndpointError::BufferOverflow) => {
                        // not much we can do if this happens, just ignore the error
                    }
                }
            }
            //r = class.read_packet(&mut read_buf).fuse() => {
            r = message_parser.read(&mut class).fuse() => {
                match r {
                    Ok(Some(message)) => {
                        log::info!("Got message!!!: {:?}", message);
                        if IN.try_send(message).is_err() {
                            log::error!("USB serial in buffer is full");
                        }
                    }
                    Ok(None) |
                    Err(EndpointError::Disabled) => wait_connection(&mut class).await,
                    Err(EndpointError::BufferOverflow) => {
                        // wtf, this shouldn't happen?
                        panic!("usb serial buffer overflow on read");
                    }
                }
            }
        }
    }
}

/// Send a [DeviceMsg] over the serial connection.
///
/// If the OUT buffer is full, the message may be partially dropped.
///
/// If no serial connection is active, the message may not be ever be sent.
pub fn serial_send(message: &DeviceMsg<'_>) {
    struct UsbWriter;
    impl msgpck::Write for UsbWriter {
        fn write_all(&mut self, bytes: &[u8]) -> Result<(), PackErr> {
            OUT.try_write(bytes).map_err(|_| PackErr::BufferOverflow)?;
            Ok(())
        }
    }

    let _ = message.pack_with_writer(&mut UsbWriter);
}

impl LogOutput for UsbSerial {
    fn log(&self, record: &TimestampedRecord) {
        let ms = record.timestamp.as_millis();
        let level = record.metadata().level();

        let record = LogRecord {
            timestamp: ms,
            level: DisplayPack(level),
            message: DisplayPack(record.args()),
        };

        serial_send(&DeviceMsg::Log(record));
    }
}

pub struct MessageParser<'a> {
    pub buf: &'a mut [u8],
    pub len: usize,
}

impl<'buf> MessageParser<'buf> {
    pub fn new(buf: &'buf mut [u8]) -> Self {
        Self { buf, len: 0 }
    }

    pub async fn read(
        &mut self,
        class: &mut CdcAcmClass<'static, Driver<'static, USB>>,
    ) -> Result<Option<HostMsg>, EndpointError> {
        loop {
            // try to parse messages from the buffer
            if self.len > 0 {
                log::debug!("buf: {:x?}", &self.buf[..self.len]);
            }
            let mut reader = &mut &self.buf[..self.len];
            match HostMsg::unpack(&mut reader) {
                Ok(r) => {
                    // remove the decoded bytes from buf
                    if reader.is_empty() {
                        self.len = 0;
                    } else {
                        let bytes_read = self.len - reader.len();
                        self.buf.rotate_left(bytes_read);
                        self.len -= bytes_read;
                    }

                    log::debug!("received message: {r:?}");

                    return Ok(Some(r));
                }

                // we probably have not gotten the entire message yet, go back to reading bytes.
                // if the message is corrupted, we will eventually hit MESSAGE_TIMEOUT or
                // max buffer size.
                Err(UnpackErr::UnexpectedEof) => {}

                // on any other error, the message is corrupt. clear the buffer.
                Err(_e) => {
                    log::error!("{:?}", _e);
                    self.len = 0;
                }
            };

            // if buffer is not empty, this future will sleep until the pending message times out
            let buf_is_empty = self.len == 0;
            let timeout = async {
                if buf_is_empty {
                    pending().await
                } else {
                    const MESSAGE_TIMEOUT: Duration = Duration::from_millis(30);
                    Timer::after(MESSAGE_TIMEOUT).await
                }
            };

            // try to read some bytes from the file
            let n = select_biased! {
                n = class.read_packet(&mut self.buf[self.len..]).fuse() => n?,
                _ = timeout.fuse() => {
                    log::debug!("clearing buffer");
                    self.len = 0;
                    continue;
                }
            };

            // make sure we're not just reading garbage forever
            if self.len >= self.buf.len() {
                log::debug!("max message size exceeded");
                self.len = 0;
                continue;
            }

            self.len += n;

            log::info!("read {} bytes, buf: {:x?}", n, &self.buf[..self.len]);

            // exit on eof
            if n == 0 {
                log::error!("read 0 bytes, fuck.");
                return Ok(None);
            }
        }
    }
}
