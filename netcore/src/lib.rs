mod net;

pub use net::{NetEvent, NetServer, Source};

#[repr(C)]
pub enum EntryCode {
    New,
    Restarted { initializer: Box<[u8]> },
}

#[repr(C)]
pub enum ExitCode {
    PleaseRestart { initializer: Box<[u8]> },
    Exit,
}
