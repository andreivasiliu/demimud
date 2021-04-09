# clicore

`clicore` is similar to `netcore`, but provides no network server; instead, it runs a simple REPL-like CLI which simply listens on stdin and outputs colored text to stdout.

It is single-player, and the time is always paused, with time advancing a tick only once a line is read via stdin.

Just like `netcore`, it loads the MUD logic from `mudlib`, but it is statically linked to it and cannot hot-swap code.

It can be compiled to WASI. Type `demimud` at https://webassembly.sh/, and see https://wapm.io/package/whyte/demimud.
