use std::time::Duration;

use libloading::{library_filename, Symbol};
use netcore::{EntryCode, ExitCode, NetServer};

fn main() {
    let mut net_server = NetServer::new("127.0.0.1:23".parse().unwrap());

    net_server.set_tick(Duration::from_millis(1000));

    let mut entry_initializer: Option<Box<[u8]>> = None;

    loop {
        let mudlib_original = library_filename("target/debug/mudlib");
        let mudlib_backup = library_filename("target/debug/backup_mudlib");
        let mudlib = library_filename("target/debug/live_mudlib");
        
        std::fs::copy(mudlib_original, &mudlib).expect("Could not copy mudlib");

        let entry_code = match entry_initializer.take() {
            Some(initializer) => EntryCode::Restarted { initializer },
            None => EntryCode::New,
        };

        let exit_code = unsafe {
            let library = libloading::Library::new(&mudlib).expect("Couldn't load library");

            let do_things: Symbol<fn(&mut NetServer, EntryCode) -> ExitCode>;
            do_things = library.get(b"do_things").unwrap();

            do_things(&mut net_server, entry_code)
        };

        match exit_code {
            ExitCode::Exit => break,
            ExitCode::PleaseRestart { initializer } => {
                // It was good enough to trigger a restart, so back it up
                std::fs::copy(mudlib, mudlib_backup).expect("Couldn't create backup");

                entry_initializer = Some(initializer);
                continue;
            }
        }
    }
}
