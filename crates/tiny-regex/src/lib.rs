#![doc = include_str!("../README.md")]

#![no_std]

mod api;
pub mod raw;

pub use api::{Match, Matches, Regex, RegexBuf, TinyRegex, DEFAULT_CCL, DEFAULT_MATCH_TEXT_LEN, DEFAULT_MEMO, DEFAULT_NODES};


