use super::layout::vm_layout_constants::*;
use crate::util::constants::*;
use crate::util::Address;

////////// FIXME //////////////
#[cfg(any(target_pointer_width = "32", feature = "force_32bit_heap_layout"))]
pub const HEAP_LAYOUT_32BIT: bool = true;
#[cfg(all(target_pointer_width = "64", not(feature = "force_32bit_heap_layout")))]
pub const HEAP_LAYOUT_32BIT: bool = false; // FIXME SERIOUSLY
pub const HEAP_LAYOUT_64BIT: bool = !HEAP_LAYOUT_32BIT;

#[derive(Clone, Copy, Debug)]
pub enum VMRequest {
    RequestDiscontiguous,
    RequestFixed {
        start: Address,
        extent: usize,
        top: bool,
    },
    RequestExtent {
        extent: usize,
        top: bool,
    },
    RequestFraction {
        frac: f32,
        top: bool,
    },
}

impl VMRequest {
    pub fn is_discontiguous(&self) -> bool {
        match self {
            VMRequest::RequestDiscontiguous { .. } => true,
            _ => false,
        }
    }

    pub fn common64bit(top: bool) -> Self {
        VMRequest::RequestExtent {
            extent: MAX_SPACE_EXTENT,
            top,
        }
    }

    pub fn discontiguous() -> Self {
        if HEAP_LAYOUT_64BIT {
            return Self::common64bit(false);
        }
        VMRequest::RequestDiscontiguous
    }

    pub fn fixed_size(mb: usize) -> Self {
        if HEAP_LAYOUT_64BIT {
            return Self::common64bit(false);
        }
        VMRequest::RequestExtent {
            extent: mb << LOG_BYTES_IN_MBYTE,
            top: false,
        }
    }

    pub fn fraction(frac: f32) -> Self {
        if HEAP_LAYOUT_64BIT {
            return Self::common64bit(false);
        }
        VMRequest::RequestFraction { frac, top: false }
    }

    pub fn high_fixed_size(mb: usize) -> Self {
        if HEAP_LAYOUT_64BIT {
            return Self::common64bit(true);
        }
        VMRequest::RequestExtent {
            extent: mb << LOG_BYTES_IN_MBYTE,
            top: true,
        }
    }

    pub fn fixed_extent(extent: usize, top: bool) -> Self {
        if HEAP_LAYOUT_64BIT {
            return Self::common64bit(top);
        }
        VMRequest::RequestExtent { extent, top }
    }
}
