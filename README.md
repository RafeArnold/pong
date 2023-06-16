# pong
play pong online, entirely within your terminal.

## usage

to start a new game:

running with [cargo](https://rustup.rs/) directly:
```
cargo r --bin client new
```
this will print a lobby id to the screen that can be used by another user to join the game.

to join an existing game:

running with [cargo](https://rustup.rs/) directly:
```
cargo r --bin client join <LOBBY_ID>
```
