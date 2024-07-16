use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct RonEdit<T> {
    pub t: T,
    pub ron: String,
    pub error: String,
}

impl<T: Serialize> RonEdit<T> {
    pub fn new(t: T) -> Self {
        RonEdit {
            ron: ron::ser::to_string_pretty(&t, Default::default()).unwrap(),
            error: String::new(),
            t,
        }
    }
}

impl<T: Default + Serialize> Default for RonEdit<T> {
    fn default() -> Self {
        RonEdit::new(Default::default())
    }
}
