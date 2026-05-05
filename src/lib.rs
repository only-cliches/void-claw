#![allow(
    clippy::bind_instead_of_map,
    clippy::cmp_owned,
    clippy::collapsible_if,
    clippy::derivable_impls,
    clippy::double_ended_iterator_last,
    clippy::doc_lazy_continuation,
    clippy::field_reassign_with_default,
    clippy::match_like_matches_macro,
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::while_let_loop
)]

pub mod activity;
pub mod agents;
pub mod ca;
pub mod cli;
pub mod config;
pub mod container;
pub mod exec;
pub mod init;
pub mod manager;
pub mod new_project;
pub mod passthrough;
pub mod proxy;
pub mod rules;
pub mod server;
pub mod shared_config;
pub mod state;
pub mod telemetry;
pub mod tui;
