# pong
play pong online, entirely within your terminal.

a server and terminal client implementation of pong, built entirely in rust. client and server communicate over raw tcp using an _extremely_ compact custom protocol (most messages are one or two bytes in length).

## usage

to play a game of pong, the client can be downloaded from this repository's [releases](https://github.com/RafeArnold/pong/releases/) or ran with [cargo](https://rustup.rs/).
the below commands are shown using cargo.
if instead you've downloaded a release binary, just omit the `cargo run --bin` prefix and replace `client` with the path to the binary on your filesystem.

to start a new game:
```
$ cargo run --bin client new
```
this will print a lobby id to the screen that can be used by another user to join the game.

to join an existing game:
```
$ cargo run --bin client join <LOBBY_ID>
```

by default, the client is configured to connect to my server.
if you are running your own pong server that you want the client to connect to, set `PONG_SERVER_ADDR` on your environment with the format `[IPv4]:[PORT]` (e.g. `0.0.0.0:8080`) before running the client binary.
