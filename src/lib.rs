//! A lightweight KCP implementation using smol async runtime
//! 
//! This library provides a minimal KCP (reliable UDP) implementation
//! designed for resource-constrained environments like OpenWrt.

pub use config::{KcpConfig, KcpNoDelayConfig};
pub use listener::KcpListener;
pub use stream::KcpStream;

mod config;
mod listener;
mod socket;
mod stream;

pub use kcp::{Error as KcpError, KcpResult};