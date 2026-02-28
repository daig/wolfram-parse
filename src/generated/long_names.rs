//! Unicode long name character data and mappings
//!
//! Generated from data/long_names.json via build.rs

use crate::read::code_point::CodePoint;
use crate::tokenize::TokenKind::{self, *};

include!(concat!(env!("OUT_DIR"), "/generated/long_names_constants.rs"));
include!(concat!(env!("OUT_DIR"), "/generated/long_names_mappings.rs"));
include!(concat!(env!("OUT_DIR"), "/generated/long_names_character_classes.rs"));
include!(concat!(env!("OUT_DIR"), "/generated/long_names_character_sets.rs"));
include!(concat!(env!("OUT_DIR"), "/generated/long_names_operators.rs"));
