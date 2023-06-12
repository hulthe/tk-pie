use core::time::Duration;

use serde::{Deserialize, Serialize};
use tgnt::{button::Button, keys::Key};

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
    use tgnt::button::Modifier;

    use super::*;

    /// A usb keyboard button was pressed or released.
    ///
    /// This is a lower-level event than a [SwitchEvent], as things like ModTap and Compose are
    /// converted to Presses and Releases.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Event {
        PressKey(Key),
        ReleaseKey(Key),
        PressMod(Modifier),
        ReleaseMod(Modifier),
    }
}

/// A keyboard half.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Half {
    Left,
    Right,
}
