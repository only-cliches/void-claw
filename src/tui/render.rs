#![allow(unused_imports)]

use super::*;
use crate::state::DecisionKind;
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::term::cell::Flags as TermFlags;
use alacritty_terminal::vte::ansi::{Color as AnsiColor, NamedColor, Rgb};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use std::time::{SystemTime, UNIX_EPOCH};

mod overlays;
mod root;
mod sidebar;
mod terminal;

pub(crate) use overlays::*;
pub use root::render;
pub(crate) use root::render_scrollbar;
pub(crate) use sidebar::*;
pub(crate) use terminal::*;

mod panes {
    use super::*;

    #[path = "build.rs"]
    mod build;
    #[path = "text.rs"]
    mod text;

    pub(crate) use build::*;
    pub(crate) use text::*;
}

pub(crate) use panes::*;
