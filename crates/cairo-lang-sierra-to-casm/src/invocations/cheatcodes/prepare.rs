use std::ops::Deref;

use cairo_lang_casm::{builder::CasmBuilder, casm_build_extend};

use super::{CompiledInvocation, CompiledInvocationBuilder, InvocationError};
use crate::invocations::{
    add_input_variables, get_non_fallthrough_statement_id, CostValidationInfo,
};

pub fn build_prepare(
    builder: CompiledInvocationBuilder<'_>,
) -> Result<CompiledInvocation, InvocationError> {
    let failure_handle_statement_id = get_non_fallthrough_statement_id(&builder);
    let refs = builder.try_get_refs::<2>()?;

    let mut optional_class_hash = None;
    if let [maybe_class_hash] = refs[0].cells.deref() {
        optional_class_hash = Some(maybe_class_hash);
    }
    let class_hash = optional_class_hash.ok_or(InvocationError::InvalidGenericArg)?;

    let [calldata_start, calldata_end] = refs[3].try_unpack()?;

    let mut casm_builder = CasmBuilder::default();
    add_input_variables! {casm_builder,
        deref class_hash;
        deref calldata_start;
        deref calldata_end;
    };

    casm_build_extend! {casm_builder,
        tempvar err_code;
        hint Invoke {
            class_hash: class_hash,
            calldata_start: calldata_start,
            calldata_end: calldata_end
        } into {err_code: err_code};
        jump Failure if err_code != 0;
    };

    Ok(builder.build_from_casm_builder(
        casm_builder,
        [
            ("Fallthrough", &[], None),
            ("Failure", &[&[err_code]], Some(failure_handle_statement_id)),
        ],
        CostValidationInfo { range_check_info: None, extra_costs: None },
    ))
}
