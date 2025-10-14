# Miniscop: Investigate Together!

This project was planned to be a multiplayer recreation of Petscop made with Bevy. I used it as a testing playground for learning Rust.

This repository contains code for:
- A server that can be run from the command line
    - Requires a valid domain and certificate
- A client that loads into the Gift Plane.

You can compile the client yourself with
`cargo run --package miniscop --bin client --features bevy/dynamic_linking --profile dev`

The required CLI args for the server are [here](https://github.com/KingRocco21/Miniscop-Bevy/blob/main/src/bin/server/main.rs#L17).

# Credits
Thanks to [Openscop](https://github.com/TechMan06/Openscop) for letting me use the guardian sprite.

Thanks to Tony for the [Petscop Soundtrack and SFX](https://petscop.bandcamp.com/album/petscop-soundtrack).
