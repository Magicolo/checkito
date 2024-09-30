pub use checkito::{
    check::{Cause, Error},
    utility::Nudge,
    *,
};
use std::{error, result};

pub type Result = result::Result<(), Box<dyn error::Error>>;
