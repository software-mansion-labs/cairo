use cairo_lang_casm::builder::CasmBuilder;
use cairo_lang_casm::casm_build_extend;

use crate::invocations::{
    add_input_variables, CompiledInvocation, CompiledInvocationBuilder, CostValidationInfo,
    InvocationError,
};

pub fn build_start_spoof(
    builder: CompiledInvocationBuilder<'_>,
) -> Result<CompiledInvocation, InvocationError> {
    let [
        contract_address,
        version,
        set_version,
        account_contract_address,
        set_account_contract_address,
        max_fee,
        set_max_fee,
        signature,
        set_signature,
        transaction_hash,
        set_transaction_hash,
        chain_id,
        set_chain_id,
        nonce,
        set_nonce,
    ] = builder.try_get_refs::<15>()?;

    let contract_address = contract_address.try_unpack_single()?;
    let version = version.try_unpack_single()?;
    let set_version = set_version.try_unpack_single()?;
    let account_contract_address = account_contract_address.try_unpack_single()?;
    let set_account_contract_address = set_account_contract_address.try_unpack_single()?;
    let max_fee = max_fee.try_unpack_single()?;
    let set_max_fee = set_max_fee.try_unpack_single()?;
    let set_signature = set_signature.try_unpack_single()?;
    let transaction_hash = transaction_hash.try_unpack_single()?;
    let set_transaction_hash = set_transaction_hash.try_unpack_single()?;
    let chain_id = chain_id.try_unpack_single()?;
    let set_chain_id = set_chain_id.try_unpack_single()?;
    let nonce = nonce.try_unpack_single()?;
    let set_nonce = set_nonce.try_unpack_single()?;

    let [signature_start, signature_end]= signature.try_unpack()?;

    let mut casm_builder = CasmBuilder::default();

    add_input_variables! {casm_builder,
        deref contract_address;
        deref version;
        deref set_version;
        deref account_contract_address;
        deref set_account_contract_address;
        deref max_fee;
        deref set_max_fee;
        deref signature_start;
        deref signature_end;
        deref set_signature;
        deref transaction_hash;
        deref set_transaction_hash;
        deref chain_id;
        deref set_chain_id;
        deref nonce;
        deref set_nonce;
    }

    casm_build_extend! {casm_builder,
        hint ProtostarHint::StartSpoof {
            contract_address: contract_address,
            version: version,
            set_version: set_version,
            account_contract_address: account_contract_address,
            set_account_contract_address: set_account_contract_address,
            max_fee: max_fee,
            set_max_fee: set_max_fee,
            signature_data_start: signature_start,
            signature_data_end: signature_end,
            set_signature: set_signature,
            transaction_hash: transaction_hash,
            set_transaction_hash: set_transaction_hash,
            chain_id: chain_id,
            set_chain_id: set_chain_id,
            nonce: nonce,
            set_nonce: set_nonce
        } into {};
        ap += 0;
    }

    Ok(builder.build_from_casm_builder(
        casm_builder,
        [("Fallthrough", &[], None)],
        CostValidationInfo { range_check_info: None, extra_costs: None },
    ))
}
