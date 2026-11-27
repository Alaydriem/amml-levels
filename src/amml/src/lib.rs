//! @alaydriem/levels as an AMML mod: reads the surface light around the player each tick through the loader and draws
//! it as flat label meshes in the level render, driven by levels-core and registered with the mod manager.

#![deny(unsafe_code)]

pub mod levels_mod;
pub mod services;
pub mod types;

pub use levels_mod::LevelsMod;

amml_api::amml_mod!(LevelsMod);
