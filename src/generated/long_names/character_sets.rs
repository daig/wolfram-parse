//! Additional character mappings

use super::constants::*;

// Minimal ASCII replacements map for core functionality
pub static ASCII_REPLACEMENTS_MAP: &[(char, &[&str])] = &[
    (CODEPOINT_LONGNAME_NOT, ["!", ].as_slice()),
    (CODEPOINT_LONGNAME_DEGREE, ["Degree", ].as_slice()),
    (CODEPOINT_LONGNAME_PLUSMINUS, ["+-", ].as_slice()),
    (CODEPOINT_LONGNAME_CENTERDOT, [".", ].as_slice()),
    (CODEPOINT_LONGNAME_TIMES, ["*", ].as_slice()),
    (CODEPOINT_LONGNAME_DIVIDE, ["/", ].as_slice()),
];