//!
//! # OT 算法（Operational Transform）实现
//! > 实现上参考了 [Operational-Transformation/ot.js](https://github.com/Operational-Transformation/ot.js/blob/master/lib/text-operation.js)

mod error;
mod operation;
mod text;

pub use error::OperationError;
pub use text::TextOperation;
