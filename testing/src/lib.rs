//! Testing & Simulation Module for Soroban contracts
//! 
//! Provides comprehensive test harnesses, mocks, fuzzing helpers, and simulation tools
//! for all contracts in the alian_structure-contracts repository.

#![no_std]

pub mod mocks;
pub mod helpers;
pub mod simulation;
pub mod fuzzing;
pub mod examples;

pub use mocks::*;
pub use helpers::*;
pub use simulation::*;
pub use fuzzing::*;