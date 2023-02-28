use cairo_lang_sierra::extensions::cheatcodes::CheatcodesConcreteLibFunc;

use self::{declare::build_declare, roll::build_roll, start_prank::build_start_prank};

use super::{CompiledInvocation, CompiledInvocationBuilder};
use crate::invocations::InvocationError;

mod declare;
mod prepare;
mod roll;
mod start_prank;

/// Builds instructions for Sierra array operations.
pub fn build(
    libfunc: &CheatcodesConcreteLibFunc,
    builder: CompiledInvocationBuilder<'_>,
) -> Result<CompiledInvocation, InvocationError> {
    match libfunc {
        CheatcodesConcreteLibFunc::Roll(_) => build_roll(builder),
        CheatcodesConcreteLibFunc::Declare(_) => build_declare(builder),
        CheatcodesConcreteLibFunc::StartPrank(_) => build_start_prank(builder),
        CheatcodesConcreteLibFunc::Prepare(_) => build_prepare(builder),
    }
}
