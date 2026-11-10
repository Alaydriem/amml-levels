//! @alaydriem/levels as an AMML mod: reads the surface light around the player each tick and draws it as flat
//! label meshes in the level render, driven by levels-core and registered with the mod manager.

#![deny(unsafe_code)]

pub mod detours;
pub mod errors;
pub mod game;
pub mod levels_mod;
pub mod services;
pub mod types;

pub use levels_mod::LevelsMod;

amml_sdk::amml_mod!(LevelsMod);
