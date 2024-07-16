mod port;
mod scan;
mod state;

pub use port::connect_to_serial;
pub use scan::scan_for_serial;
pub use state::SerialState;
