//! Communication between keyboard halves.

use core::mem::size_of;

use bytemuck::{bytes_of, AnyBitPattern, NoUninit};
use crc_any::CRCu16;
use embassy_sync::channel::Channel;
use rend::u16_le;
use serde::{Deserialize, Serialize};

use crate::event::switch;
use crate::usb::UsbEvent;
use crate::util::CS;

#[cfg(target_arch = "arm")]
pub mod uart;

/// Channel for [UsbEvent]s to be sent on the uart line.
pub static USB_EVENTS_OUT: Channel<CS, UsbEvent, 8> = Channel::new();

/// Send messages this many times to decrease likelyhood of missed messages.
pub const RESEND_COUNT: usize = 3;

/// How old a sequence number is allowed to be before we think something is wrong.
pub const BAD_SEQNUM_THRESHOLD: u16 = 10;

/// Receive buffer capuacity.
pub const RX_BUF_CAP: usize = 1024;

/// Size of a [Header].
pub const HEADER_LEN: usize = size_of::<Header>();

/// Size of a packet body.
pub const MAX_BODY_LEN: usize = u8::MAX as usize;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Message {
    KeyboardEvent(switch::Event),
    UsbEvent(UsbEvent),
}

/// The header of a UART packet.
#[repr(C)]
#[derive(Clone, Copy, Debug, NoUninit, AnyBitPattern)]
pub struct Header {
    /// The length of the payload.
    len: u8,

    /// An arbitrary value to feed the crc, should be different for each message.
    random: u8,

    /// A sequence number, each new message sent must increment this by 1.
    seqnum: u16_le,

    // CRC of the body.
    body_crc: u16_le,

    // CRC of the header. When calculated, this field must be set to 0.
    header_crc: u16_le,
}

impl Header {
    /// Construct a new valid header.
    pub fn new(random: u8, seqnum: u16, body: &[u8]) -> Self {
        let len: u8 = body.len().try_into().expect("Packet length exceed maximum");

        let mut header = Self {
            len,
            random,
            seqnum: seqnum.into(),
            body_crc: Self::calculate_body_crc(body).into(),
            header_crc: 0.into(),
        };

        let mut crc = CRCu16::crc16();
        crc.digest(bytes_of(&header));
        header.header_crc = crc.get_crc().into();

        header
    }

    pub fn calculate_header_crc(&self) -> u16 {
        let mut crc = CRCu16::crc16();

        let header = Header {
            header_crc: 0.into(),
            ..*self
        };

        crc.digest(bytes_of(&header));
        crc.get_crc()
    }

    pub fn calculate_body_crc(body: &[u8]) -> u16 {
        let mut crc = CRCu16::crc16();
        crc.digest(body);
        crc.get_crc()
    }

    pub fn valid_header_crc(&self) -> bool {
        let expected_crc = self.calculate_header_crc();
        self.header_crc.value() == expected_crc
    }

    pub fn valid_body_crc(&self, body: &[u8]) -> bool {
        let expected_crc = Self::calculate_body_crc(body);
        self.body_crc.value() == expected_crc
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum SeqnumResult {
    OldSeqnum,
    NewSeqnum,
    Desync,
}

impl SeqnumResult {
    /// Whether a newly received message should be discarded based on the seqnum.
    pub const fn should_discard_message(&self) -> bool {
        matches!(self, SeqnumResult::OldSeqnum)
    }
}

/// Returns true if `new` is considered greater than `known`. Takes wrapping into account.
pub fn compare_seqnums(new: u16, known: u16) -> SeqnumResult {
    let diff = known.wrapping_sub(new);

    /// if diff is greater than this, `new` is indeed a new seqnum
    const NEW_SEQNUM_THRESHOLD: u16 = u16::MAX - BAD_SEQNUM_THRESHOLD + 1;

    match diff {
        ..=BAD_SEQNUM_THRESHOLD => SeqnumResult::OldSeqnum,
        NEW_SEQNUM_THRESHOLD.. => SeqnumResult::NewSeqnum,
        _ => SeqnumResult::Desync,
    }
}

#[cfg(all(target_arch = "x86_64", test))]
mod tests {
    use crate::hemicom::{compare_seqnums, SeqnumResult, BAD_SEQNUM_THRESHOLD};

    use super::Header;

    #[test]
    fn seqnum_comparison() {
        use SeqnumResult::*;
        let max = u16::MAX;
        assert_eq!(compare_seqnums(0, 0), OldSeqnum);
        assert_eq!(compare_seqnums(0, 1), OldSeqnum);
        assert_eq!(compare_seqnums(0, BAD_SEQNUM_THRESHOLD), OldSeqnum);
        assert_eq!(compare_seqnums(0, BAD_SEQNUM_THRESHOLD + 1), Desync);
        assert_eq!(compare_seqnums(0, BAD_SEQNUM_THRESHOLD + 12345), Desync);
        assert_eq!(compare_seqnums(1, 0), NewSeqnum);
        assert_eq!(compare_seqnums(1, max), NewSeqnum);
        assert_eq!(compare_seqnums(max, max), OldSeqnum);
        assert_eq!(compare_seqnums(max, max - 1), NewSeqnum);
        assert_eq!(compare_seqnums(max, max - BAD_SEQNUM_THRESHOLD), NewSeqnum);
        assert_eq!(compare_seqnums(max, max - BAD_SEQNUM_THRESHOLD - 1), Desync);
        assert_eq!(compare_seqnums(max, max / 2), Desync);
        assert_eq!(compare_seqnums(0, max / 2), Desync);
    }

    #[test]
    fn header_crc() {
        let mock_mody = &[1u8, 2, 3, 4, 5, 6];
        let header = Header::new(mock_mody.len() as u8, 123, 456).with_valid_crc(mock_mody);
        assert!(header.valid_header_crc());
        assert!(header.valid_body_crc(mock_mody));
    }
}
