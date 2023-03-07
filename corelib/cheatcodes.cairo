extern fn roll(address: felt, caller_address: felt) -> Result::<(), felt> nopanic;

extern fn warp(blk_timestamp: felt, target_contract_address: felt) -> Result::<(), felt> nopanic;

extern fn declare(contract: felt) -> Result::<felt, felt> nopanic;

struct PreparedContract {
    constructor_calldata: Array::<felt>,
    contract_address: felt,
    class_hash: felt,
}

extern fn prepare_tp(
    class_hash: felt, calldata: Array::<felt>
) -> Result::<(Array::<felt>, felt, felt), felt> nopanic;

fn prepare(class_hash: felt, calldata: Array::<felt>) -> Result::<PreparedContract, felt> nopanic {
    match prepare_tp(class_hash, calldata) {
        Result::Ok((
            constructor_calldata, contract_address, class_hash
        )) => Result::<PreparedContract,
        felt>::Ok(
            PreparedContract {
                constructor_calldata: constructor_calldata,
                contract_address: contract_address,
                class_hash: class_hash,
            }
        ),
        Result::Err(x) => Result::<PreparedContract, felt>::Err(x)
    }
}

extern fn start_prank(
    caller_address: felt, target_contract_address: felt
) -> Result::<(), felt> nopanic;

extern fn invoke(
    contract_address: felt, entry_point_selector: felt, calldata: Array::<felt>
) -> Result::<(), felt> nopanic;

extern fn mock_call(
    contract_address: felt, entry_point_selector: felt, response: Array::<felt>
) -> Result::<(), felt> nopanic;
