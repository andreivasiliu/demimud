use bevy::prelude::*;

use netcore::{EntryCode, ExitCode, NetServer};

#[no_mangle]
pub extern fn do_things(net_server: &mut NetServer, entry_code: EntryCode) -> ExitCode {
    match entry_code {
        EntryCode::New => (),
        EntryCode::Restarted { initializer: _ } => (),
    }

    App::build()
        .add_plugin(bevy::log::LogPlugin::default())
        .add_system(hello_world_system.system())
        .run();

    for _ in 0..10 {
        println!("Tick: {:?}", net_server.receive_event().1);
    }

    println!("Done!");

    ExitCode::PleaseRestart { initializer: Vec::new().into_boxed_slice() }
}

fn hello_world_system() {
    println!("hello world");
}
