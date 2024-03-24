use alloc::vec::Vec;
use msgpck::{MsgPack, MsgUnpack};
use serde::{Deserialize, Serialize};

use crate::button::Button;

#[derive(Default, Debug, Serialize, Deserialize, Clone, MsgPack, MsgUnpack)]
pub struct Layer {
    pub buttons: Vec<Button>,
}

pub type Layers = Vec<Vec<Layer>>;
