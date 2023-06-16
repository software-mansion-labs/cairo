use cairo_lang_casm::builder::CasmBuilder;
use cairo_lang_casm::casm_build_extend;

use crate::invocations::{
    add_input_variables, CompiledInvocation, CompiledInvocationBuilder, CostValidationInfo,
    InvocationError,
};

pub fn build_stop_spoof(
    builder: CompiledInvocationBuilder<'_>,
) -> Result<CompiledInvocation, InvocationError> {
    let [contract_address] = builder.try_get_refs::<1>()?;

    let contract_address = contract_address.try_unpack_single()?;

    let mut casm_builder = CasmBuilder::default();

    add_input_variables! {casm_builder, deref contract_address;}

    casm_build_extend! {casm_builder,
        hint ProtostarHint::StopSpoof {
            contract_address: contract_address
        } into {};
        ap += 0;
    }

    Ok(builder.build_from_casm_builder(
        casm_builder,
        [("Fallthrough", &[], None)],
        CostValidationInfo { range_check_info: None, extra_costs: None },
    ))
}
