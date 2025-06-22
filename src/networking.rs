use bincode::config::Configuration;
use bincode::Encode;
use bincode::{config, Decode};

pub const PACKET_CONFIG: Configuration = config::standard();
#[derive(Encode, Decode, Debug)]
pub enum Packet {
    PlayerPosition { x: f32, y: f32, z: f32 },
    Other,
}
