#![feature(windows_by_handle)]
#![forbid(unsafe_code)]

extern crate failure;
extern crate jwalk;

mod aggregate;
mod common;
mod inodefilter;

pub mod traverse;

pub use aggregate::aggregate;
pub use common::*;
pub(crate) use inodefilter::InodeFilter;
