use crate::{button::Button, keys::Key};
use core::time::Duration;
use serde::{Deserialize, Serialize};

pub mod switch {
    use super::*;

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
    /// A switch was pressed or released
    ///
    /// This event is triggered by tasks that monitor switches.
    pub struct Event {
        /// The keyboard half that triggered the event.
        pub source: Half,

        /// The index of the button that triggered the event.
        pub source_button: usize,

        pub kind: EventKind,
    }

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
    pub enum EventKind {
        Press {
            button: Button,
        },
        Release {
            button: Button,

            /// The duration that the button was held down for
            after: Duration,
        },
    }
}

pub mod button {
    use super::*;
    use crate::button::Modifier;

    /// A usb keyboard button was pressed or released.
    ///
    /// This is a lower-level event than a [switch::Event], as things like ModTap and Compose are
    /// converted to Presses and Releases.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Event {
        PressKey(Key),
        ReleaseKey(Key),
        PressMod(Modifier),
        ReleaseMod(Modifier),
        Wait,
    }
}

/// A keyboard half.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Half {
    Left,
    Right,
}
