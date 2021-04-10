use std::{path::PathBuf, time::Duration};

use libloading::{library_filename, Symbol};
use netcore::{EntryCode, ExitCode, NetServer};

fn main() {
    let address = "0.0.0.0:23".parse().unwrap();
    println!("Starting server on {}", address);

    let mut net_server = NetServer::new(address);

    net_server.set_tick(Duration::from_millis(1000));

    let mut entry_initializer: Option<Box<[u8]>> = None;

    let bin_path = std::env::current_exe().expect("Could not get path to executable");

    let mudlib_name = PathBuf::from(library_filename("mudlib"));
    println!(
        "Using {} next to bin path: {}",
        mudlib_name.display(),
        bin_path.display()
    );
    println!(
        "Using data inside current directory: {}",
        std::env::current_dir().unwrap().display()
    );

    let bin_dir = bin_path
        .parent()
        .expect("Could not get directory fo executable");

    let mudlib_original = bin_dir.join(library_filename("mudlib"));
    let mudlib_backup = bin_dir.join(library_filename("backup_mudlib"));
    let mudlib = bin_dir.join(library_filename("live_mudlib"));

    loop {
        // On Windows a live .dll file is locked and cannot be written to, so
        // copy it to allow cargo to build a new one.
        std::fs::copy(&mudlib_original, &mudlib).expect("Could not copy mudlib");

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
                // It was good enough to trigger a restart, so back it up.
                std::fs::copy(&mudlib, &mudlib_backup).expect("Couldn't create backup");

                entry_initializer = Some(initializer);
                continue;
            }
        }
    }
}
