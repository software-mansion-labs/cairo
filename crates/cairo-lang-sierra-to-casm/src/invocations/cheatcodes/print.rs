use std::ops::Deref;

use cairo_lang_casm::builder::CasmBuilder;
use cairo_lang_casm::casm_build_extend;

use super::{CompiledInvocation, CompiledInvocationBuilder, InvocationError};
use crate::invocations::{
    add_input_variables, get_non_fallthrough_statement_id, CostValidationInfo,
};

pub fn build_print(
    builder: CompiledInvocationBuilder<'_>,
) -> Result<CompiledInvocation, InvocationError> {
    let failure_handle_statement_id = get_non_fallthrough_statement_id(&builder);
    let refs = builder.try_get_refs::<2>()?;

    // data
    let [data_start, data_end] = refs[0].try_unpack()?;

    // function name
    let mut optional_format = None;
    if let [maybe_format] = refs[1].cells.deref() {
        optional_format = Some(maybe_format);
    }
    let format = optional_format.ok_or(InvocationError::InvalidGenericArg)?;

    let mut casm_builder = CasmBuilder::default();
    add_input_variables! {casm_builder,
        deref data_start;
        deref data_end;
        deref format;
    };

    casm_build_extend! {casm_builder,
        tempvar panic_data_start;
        tempvar panic_data_end;
        hint Print {
            data_start: data_start,
            data_end: data_end,
            format: format
        } into {
            panic_data_start: panic_data_start,
            panic_data_end: panic_data_end
        };
        tempvar failure = panic_data_end - panic_data_start;
        ap += 3;
        jump Failure if failure != 0;
    };

    Ok(builder.build_from_casm_builder(
        casm_builder,
        [
            ("Fallthrough", &[], None),
            ("Failure", &[&[panic_data_start, panic_data_end]], Some(failure_handle_statement_id)),
        ],
        CostValidationInfo { range_check_info: None, extra_costs: None },
    ))
}
