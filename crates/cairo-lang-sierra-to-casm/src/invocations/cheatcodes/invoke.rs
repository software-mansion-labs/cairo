use cairo_lang_casm::{builder::{CasmBuilder},  casm_build_extend};

use crate::invocations::{add_input_variables, get_non_fallthrough_statement_id, CostValidationInfo};
use super::{CompiledInvocation, CompiledInvocationBuilder, InvocationError};

pub fn build_invoke(
    builder: CompiledInvocationBuilder<'_>,
) -> Result<CompiledInvocation, InvocationError> {
    let failure_handle_statement_id = get_non_fallthrough_statement_id(&builder);
    let [contract_address, function_name, calldata] = builder.try_get_single_cells()?;

    let mut casm_builder = CasmBuilder::default();
    add_input_variables! {casm_builder,
        deref contract_address;
        deref function_name;
        deref calldata;
    };

    casm_build_extend! {casm_builder,
        tempvar err_code;
        hint Invoke {contract_address: contract_address, function_name: function_name, calldata: calldata} into {err_code: err_code};
        jump Failure if err_code != 0;
    };

    Ok(builder.build_from_casm_builder(
        casm_builder,
        [
            ("Fallthrough", &[], None),
            (
                "Failure",
                &[&[err_code]],
                Some(failure_handle_statement_id),
            ),
        ],
        CostValidationInfo {
            range_check_info: None,
            extra_costs: None,
        },
    ))

}
