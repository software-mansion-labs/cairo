extern fn cheat_roll(address: felt, caller_address: felt) -> Result::<(), felt> nopanic;

fn roll(address: felt, caller_address: felt) -> () {
    match cheat_roll(address, caller_address) {
        Result::Ok(()) => (),
        Result::Err(x) => {
            let mut data = array_new::<felt>();
            array_append::<felt>(ref data, x);
            panic(data)
        },
    }
}

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

