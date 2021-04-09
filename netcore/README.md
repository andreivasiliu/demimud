# netcore

`netcore` provides the main binary file, the network server, a library loader, and a `NetServer` object that abstracts networking into events.

It dynamically loads `mudlib`, and asks it to run its main loop, giving it a mutable referene to the `NetServer` for it to receive and send network events.

The `mudlib`'s main loop can request to be restarted, giving `netcore` an opaque object; `netcore` will unload the library, load a new version, and give it the opaque object.

The `mudlib` uses this to send a bincode-serialized `Connections` object, in order to remember about the state of open connections, players attached to each connection, and their telnet negotation state.
