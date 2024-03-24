pub mod owned {
    use alloc::string::String;
    use msgpck::{MsgPack, MsgUnpack};

    use crate::layer::Layers;

    #[derive(Debug, MsgPack, MsgUnpack)]
    pub struct LogRecord {
        /// Milliseconds since boot
        pub timestamp: u64,

        pub level: String,

        pub message: String,
    }

    /// Press the switch with the provided index.
    #[derive(Debug, MsgPack, MsgUnpack)]
    pub struct SwitchPress(pub u16);

    /// Release the switch with the provided index.
    #[derive(Debug, MsgPack, MsgUnpack)]
    pub struct SwitchRelease(pub u16);

    /// Change to the layer at the provided coordinates in the layer matrix.
    #[derive(Debug, MsgPack, MsgUnpack)]
    pub struct ChangeLayer {
        pub x: u16,
        pub y: u16,
    }

    #[derive(Debug, MsgPack, MsgUnpack)]
    pub enum DeviceMsg {
        Log(LogRecord),
        SwitchPress(SwitchPress),
        SwitchRelease(SwitchRelease),
        ChangeLayer(ChangeLayer),
        Layers(Layers),
    }

    #[derive(Debug, MsgPack, MsgUnpack)]
    pub enum HostMsg {
        GetLayers,
    }
}

pub mod borrowed {
    use msgpck::MsgPack;

    use super::owned::*;
    use crate::{layer::Layers, util::DisplayPack};

    #[derive(Debug, MsgPack)]
    pub struct LogRecord<'a> {
        /// Milliseconds since boot
        pub timestamp: u64,

        pub level: DisplayPack<log::Level>,

        pub message: DisplayPack<&'a core::fmt::Arguments<'a>>,
    }

    #[derive(Debug, MsgPack)]
    pub enum DeviceMsg<'a> {
        Log(LogRecord<'a>),
        SwitchPress(SwitchPress),
        SwitchRelease(SwitchRelease),
        ChangeLayer(ChangeLayer),
        Layers(&'a Layers),
    }
}
