pub mod boolean;
pub mod character;
pub mod number;

use super::*;
use constant::Constant;

type Result<T> = std::result::Result<(), check::Error<bool, T>>;
const COUNT: usize = 1024;
