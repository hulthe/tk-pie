use core::fmt::{self, Debug, Display};
use msgpck::{MsgPack, MsgUnpack};
use serde::{Deserialize, Serialize};

use crate::keys::Key;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, MsgPack, MsgUnpack)]
pub enum Modifier {
    LCtrl = 0x01,
    LShift = 0x02,
    LAlt = 0x04,
    LMod = 0x08,
    RCtrl = 0x10,
    RShift = 0x20,
    RAlt = 0x40,
    RMod = 0x80,
}

impl From<Modifier> for u8 {
    fn from(modifier: Modifier) -> Self {
        modifier as u8
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, MsgPack, MsgUnpack)]
#[non_exhaustive]
pub enum Button {
    Mod(Modifier),
    Key(Key),
    ModTap(Key, Modifier),
    Compose2(CompShift, Key, CompShift, Key),
    Compose3(CompShift, Key, CompShift, Key, CompShift, Key),
    Layer(LayerShift, LayerDir, u16),
    None,
}

/// Whether a key should be shift modified as part of a compose chain.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, MsgPack, MsgUnpack,
)]
pub enum CompShift {
    /// Do not shift the key.
    #[default]
    Lower,

    /// Shift the key.
    Upper,

    /// Shift the key if shift is being held down.
    Variable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, MsgPack, MsgUnpack)]
pub enum LayerDir {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, MsgPack, MsgUnpack)]
pub enum LayerShift {
    Move,
    Peek,
}

impl Display for Button {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cs = |cs: &CompShift| match cs {
            CompShift::Lower => "",
            CompShift::Upper => "⇧",
            CompShift::Variable => "⇧?",
        };

        match self {
            Button::Mod(modifier) => Debug::fmt(&modifier, f),
            Button::Key(key) => write!(f, "{key:?}"),
            Button::ModTap(key, modifier) => write!(f, "{key:?}/{modifier}"),
            Button::Compose2(cs1, k1, cs2, k2) => {
                write!(f, "⎄ {}{k1:?} {}{k2:?}", cs(cs1), cs(cs2))
            }
            Button::Compose3(cs1, k1, cs2, k2, cs3, k3) => {
                write!(f, "⎄ {}{k1:?} {}{k2:?} {}{k3:?}", cs(cs1), cs(cs2), cs(cs3))
            }
            Button::Layer(..) => write!(f, "Lr"),
            Button::None => write!(f, "Ø"),
        }
    }
}

impl Display for Modifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Modifier::LCtrl => "⎈",
            Modifier::LShift => "⇧",
            Modifier::LAlt => "⎇",
            Modifier::LMod => "◆",
            Modifier::RCtrl => "⎈",
            Modifier::RShift => "⇧",
            Modifier::RAlt => "⎇",
            Modifier::RMod => "◆",
        };

        write!(f, "{s}")
    }
}
