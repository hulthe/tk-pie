use msgpck::{MsgPack, MsgUnpack};
use serde::{Deserialize, Serialize};

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, MsgPack, MsgUnpack)]
#[non_exhaustive]
// https://usb.org/sites/default/files/hut1_3_0.pdf
pub enum Key {
    A = 0x04,
    B = 0x05,
    C = 0x06,
    D = 0x07,
    E = 0x08,
    F = 0x09,
    G = 0x0A,
    H = 0x0B,
    I = 0x0C,
    J = 0x0D,
    K = 0x0E,
    L = 0x0F,
    M = 0x10,
    N = 0x11,
    O = 0x12,
    P = 0x13,
    Q = 0x14,
    R = 0x15,
    S = 0x16,
    T = 0x17,
    U = 0x18,
    V = 0x19,
    W = 0x1A,
    X = 0x1B,
    Y = 0x1C,
    Z = 0x1D,

    /// Keyboard 1 and !
    D1 = 0x1E,
    /// Keyboard 2 and @
    D2 = 0x1F,
    /// Keyboard 3 and #
    D3 = 0x20,
    /// Keyboard 4 and $
    D4 = 0x21,
    /// Keyboard 5 and %
    D5 = 0x22,
    /// Keyboard 6 and ∧
    D6 = 0x23,
    /// Keyboard 7 and &
    D7 = 0x24,
    /// Keyboard 8 and *
    D8 = 0x25,
    /// Keyboard 9 and (
    D9 = 0x26,
    /// Keyboard 0 and )
    D0 = 0x27,

    Return = 0x28,
    Escape = 0x29,
    /// Keyboard DELETE (Backspace)
    Backspace = 0x2A,
    Tab = 0x2B,

    /// Keyboard Spacebar
    Space = 0x2C,
    /// Keyboard - and (underscore)
    Dash = 0x2D,
    /// Keyboard = and +2
    Equal = 0x2E,
    /// Keyboard [ and {
    LBracket = 0x2F,
    /// Keyboard ] and }
    RBracket = 0x30,
    /// Keyboard \and |
    BackslashPipe = 0x31,
    /// Keyboard Non-US # and  ̃
    Pound = 0x32,
    /// Keyboard ; and :
    Colon = 0x33,
    /// Keyboard ‘ and “
    Apostrophe = 0x34,
    /// Keyboard Grave Accent and Tilde
    Accent = 0x35,
    /// Keyboard , and <
    Comma = 0x36,
    /// Keyboard . and >
    Period = 0x37,
    /// Keyboard / and ?
    Slash = 0x38,
    CapsLock = 0x39,

    F1 = 0x3A,
    F2 = 0x3B,
    F3 = 0x3C,
    F4 = 0x3D,
    F5 = 0x3E,
    F6 = 0x3F,
    F7 = 0x40,
    F8 = 0x41,
    F9 = 0x42,
    F10 = 0x43,
    F11 = 0x44,
    F12 = 0x45,

    /// Keyboard PrintScreen
    PrintScreen = 0x46,
    /// Keyboard Scroll Lock
    ScrollLock = 0x47,
    /// Keyboard Pause
    Pause = 0x48,
    /// Keyboard Insert
    Insert = 0x49,
    /// Keyboard Home
    Home = 0x4A,
    /// Keyboard PageUp
    PageUp = 0x4B,
    /// Keyboard Delete Forward
    Delete = 0x4C,
    /// KeyboardEnd7
    End = 0x4D,
    /// Keyboard PageDown7
    PageDown7 = 0x4E,
    /// Keyboard RightArrow7
    RightArrow7 = 0x4F,
    /// Keyboard LeftArrow7
    LeftArrow7 = 0x50,
    /// Keyboard DownArrow7
    DownArrow7 = 0x51,
    /// Keyboard UpArrow7
    UpArrow7 = 0x52,
    /// Keypad Num Lock and Clear6
    KeypadNumLock = 0x53,
    /// Keypad /
    KeypadSlash = 0x54,
    /// Keypad *
    KeypadAsterisk = 0x55,
    /// Keypad -
    KeypadMinus = 0x56,
    /// Keypad +
    KeypadPlus = 0x57,
    /// KeypadEnter
    KeypadEnter = 0x58,
    /// Keypad 1 and End
    Keypad1 = 0x59,
    /// Keypad 2 and Down Arrow
    Keypad2 = 0x5A,
    /// Keypad 3 and PageDn
    Keypad3 = 0x5B,
    /// Keypad 4 and Left Arrow
    Keypad4 = 0x5C,
    /// Keypad 5
    Keypad5 = 0x5D,
    /// Keypad 6 and Right Arrow
    Keypad6 = 0x5E,
    /// Keypad 7 and Home
    Keypad7 = 0x5F,
    /// Keypad 8 and Up Arrow
    Keypad8 = 0x60,
    /// Keypad 9 and PageUp
    Keypad9 = 0x61,
    /// Keypad 0 and Insert
    Keypad0 = 0x62,

    /// Keypad . and Delete
    KbPeriod = 0x63,
    /// Keyboard Non-US \ and |
    NonUsBackslashPipe = 0x64,
    Application = 0x65,
    Power = 0x66,
    /// Keypad =
    KpEqual = 0x67,
    F13 = 0x68,
    F14 = 0x69,
    F15 = 0x6A,
    F16 = 0x6B,
    F17 = 0x6C,
    F18 = 0x6D,
    F19 = 0x6E,
    F20 = 0x6F,
    F21 = 0x70,
    F22 = 0x71,
    F23 = 0x72,
    F24 = 0x73,
    Execute = 0x74,
    Help = 0x75,
    Menu = 0x76,
    Select = 0x77,
    Stop = 0x78,
    Again = 0x79,
    Undo = 0x7A,
    Cut = 0x7B,
    Copy = 0x7C,
    Paste = 0x7D,
    Find = 0x7E,
    Mute = 0x7F,
    VolumeUp = 0x80,
    VolumeDown = 0x81,
    /// Keyboard Locking Caps Lock
    LockingCapsLock = 0x82,
    /// Keyboard Locking Num Lock
    LockingNumLock = 0x83,
    /// Keyboard Locking Scroll Lock
    LockingScrollLock = 0x84,
    /// Keypad ,
    KpComma = 0x85,
    /// Keypad Equal Sign
    KpEqualSIgn = 0x86,

    International1 = 0x87,
    International2 = 0x88,
    International3 = 0x89,
    International4 = 0x8A,
    International5 = 0x8B,
    International6 = 0x8C,
    International7 = 0x8D,
    International8 = 0x8E,
    International9 = 0x8F,
    Lang1 = 0x90,
    Lang2 = 0x91,
    Lang3 = 0x92,
    Lang4 = 0x93,
    Lang5 = 0x94,
    Lang6 = 0x95,
    Lang7 = 0x96,
    Lang8 = 0x97,
    Lang9 = 0x98,
    /// Keyboard Alternative Erase
    AltErase = 0x99,
    /// Keyboard SysReq/Attention
    SysReq = 0x9A,
    Cancel = 0x9B,
    Clear = 0x9C,
    Prior = 0x9D,
    /// Keyboard Return
    Return2 = 0x9E,
    /// Keyboard Separator
    Separator = 0x9F,

    /// Keyboard Out
    Out = 0xA0,
    /// Keyboard Oper
    Oper = 0xA1,
    /// Keyboard Clear/Again
    ClearAgain = 0xA2,
    /// Keyboard CrSel/Props
    CrSel = 0xA3,
    /// Keyboard ExSel
    ExSel = 0xA4,
    //
    // NOTE: the usb keyboard/keypad page allegedly har characters in the rage above A4,
    // like the ones below, but they don't seem to work
    ///// Keypad {
    //LCurly = 0xB8,
    ///// Keypad }
    //RCurly = 0xB9,
    //
    // NOTE: Not sure if these are required
    //LCtrl = 0xE0,
    //LShift = 0xE1,
    //LAlt = 0xE2,
    //RCtrl = 0xE4,
    //RShift = 0xE5,
    //RAlt = 0xE6,
}

impl From<Key> for u8 {
    fn from(key: Key) -> u8 {
        key as u8
    }
}
