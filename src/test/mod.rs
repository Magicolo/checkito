pub mod boolean;
pub mod character;
pub mod number;

use super::*;
use constant::Constant;

type Result<T> = std::result::Result<(), check::Error<T, bool>>;
const COUNT: usize = 1024;
