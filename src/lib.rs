// Hook up Clippy plugin if explicitly requested.
// You should only do this on nightly Rust.
#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]
#![cfg_attr(all(feature = "nightly", test), feature(test))]

extern crate cam;

extern crate chrono;
extern crate rand;
extern crate noise;
extern crate piston;
extern crate graphics;
extern crate glutin_window;
extern crate opengl_graphics;
#[macro_use]
extern crate gfx;
extern crate gfx_device_gl;
extern crate piston_window;
extern crate camera_controllers;
extern crate vecmath;
extern crate shader_version;
extern crate nalgebra as na;
extern crate quaternion;
#[macro_use]
extern crate slog;
extern crate slog_term;
extern crate specs;
extern crate num_traits;
extern crate wavefront_obj as obj;
extern crate ncollide;

#[cfg(all(feature = "nightly", test))]
extern crate test;

pub mod input_adapter;
pub mod globe;
pub mod types;
pub mod app;
pub mod window;
pub mod render;
pub mod simple;
pub mod cell_dweller;
pub mod movement;
pub mod system_priority;

mod spatial;
pub use spatial::Spatial;

#[cfg(test)]
mod integration_tests;
