//! Permitt core: a read-only inventory of what each AI assistant on a machine may do
//! without asking — run commands, edit files, reach the network, call MCP tools — and which
//! setting, in which file, says so.
//!
//! Host rules (`hosts/*.toml`, schema from `bbl-hostrules` plus Permitt's extensions) describe
//! each host's files, the meaning of its permission settings, its sourced defaults and its
//! precedence. The inventory resolves the files, reads them, applies the settings in
//! precedence order on top of the defaults, and produces one sheet per host (and per
//! project where the state differs), plus findings ordered by what runs without asking.
//!
//! Promises, enforced here and tested: no file is ever written; nothing leaves the machine;
//! a secret is masked the moment it is read. An unknown is said, never guessed.
#![forbid(unsafe_code)]

pub mod inventory;
pub mod model;
pub mod report;
pub mod rules;
pub mod sheet;
pub mod shell;

pub use bbl_hostrules::{env, read, secrets, Env};
pub use inventory::{Inventory, Options};
pub use rules::{HostRule, Rules};
