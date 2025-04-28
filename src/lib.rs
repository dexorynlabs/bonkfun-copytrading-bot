use anchor_lang::declare_program;

pub mod common;
pub mod core;
pub mod dex;
pub mod engine;
pub mod error;
pub mod services;

declare_program!(pump_amm);
