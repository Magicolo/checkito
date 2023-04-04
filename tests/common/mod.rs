pub use checkito::*;
use std::{error, result};

pub type Result = result::Result<(), Box<dyn error::Error>>;
pub const COUNT: usize = 1000;
