pub use crate::plan::plan_constraints::*;

pub const MOVES_OBJECTS: bool = true; //add. Reason: this one moves objects.
pub const GC_HEADER_BITS: usize = 2; //change from 0. Reason: For defining metadata
pub const GC_HEADER_WORDS: usize = 0;
pub const NUM_SPECIALIZED_SCANS: usize = 1; //add. Reason: This one actually scans stuff. ?
