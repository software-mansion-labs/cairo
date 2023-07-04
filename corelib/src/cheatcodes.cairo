use array::ArrayTrait;
use array::SpanTrait;
use clone::Clone;
use integer::Into;
use integer::TryInto;
use option::OptionTrait;
use starknet::testing::cheatcode;

#[derive(Drop, Clone)]
struct PreparedContract {
    contract_address: felt252,
    class_hash: felt252,
    constructor_calldata: Array::<felt252>,
}

#[derive(Drop, Clone)]
struct RevertedTransaction {
    panic_data: Array::<felt252>, 
}

trait RevertedTransactionTrait {
    fn first(self: @RevertedTransaction) -> felt252;
}

impl RevertedTransactionImpl of RevertedTransactionTrait {
    fn first(self: @RevertedTransaction) -> felt252 {
        *self.panic_data.at(0)
    }
}

fn declare(contract: felt252) -> Result::<felt252, felt252> {
    let span = cheatcode::<'declare'>(array![contract].span());

    let exit_code = *span[0];
    let result = *span[1];

    if exit_code == 0 {
        Result::<felt252, felt252>::Ok(result)
    } else {
        Result::<felt252, felt252>::Err(result)
    }
}

fn deploy(prepared_contract: PreparedContract) -> Result::<felt252, RevertedTransaction> {
    let PreparedContract { contract_address, class_hash, mut constructor_calldata } =
        prepared_contract;
    let mut inputs = array![contract_address, class_hash];

    let calldata_len = constructor_calldata.len().into();
    inputs.append(calldata_len);

    loop {
        match constructor_calldata.pop_front() {
            Option::Some(value) => {
                inputs.append(value);
            },
            Option::None(_) => {
                break ();
            },
        };
    };

    let outputs = cheatcode::<'deploy'>(inputs.span());
    let exit_code = *outputs[0];

    if exit_code == 0 {
        let result = *outputs[1];
        Result::<felt252, RevertedTransaction>::Ok(result)
    } else {
        // TODO: feel free to change depending on the cheatcode::<'deploy'> low level implementation of error handling
        let panic_data_len_felt = *outputs[1];
        let panic_data_len = panic_data_len_felt.try_into().unwrap();
        let mut panic_data = array![];

        let offset = 2;
        let mut i = offset;
        loop {
            if panic_data_len + offset == i {
                break ();
            }
            let value = *outputs[i];
            panic_data.append(value);
            i += 1;
        };

        Result::<felt252, RevertedTransaction>::Err(RevertedTransaction { panic_data })
    }
}

// Old code commented out for future reference - signatures etc.

// fn deploy_contract(
//     contract: felt252, calldata: @Array::<felt252>
// ) -> Result::<felt252, RevertedTransaction> {
//     let mut class_hash: Option::<felt252> = Option::None(());
//     match declare(contract) {
//         Result::Ok(x) => {
//             class_hash = Option::Some(x);
//         },
//         Result::Err(x) => {
//             let mut panic_data = ArrayTrait::new();
//             panic_data.append(x);

//             return Result::<felt252, RevertedTransaction>::Err(RevertedTransaction { panic_data });
//         }
//     }

//     let mut prepared_contract: Option::<PreparedContract> = Option::None(());
//     match prepare(class_hash.unwrap(), calldata) {
//         Result::Ok(x) => {
//             prepared_contract = Option::Some(x);
//         },
//         Result::Err(x) => {
//             let mut panic_data = ArrayTrait::new();
//             panic_data.append(x);

//             return Result::<felt252, RevertedTransaction>::Err(RevertedTransaction { panic_data });
//         }
//     }
//     deploy(prepared_contract.unwrap())
// }

// fn prepare(
//     class_hash: felt252, calldata: @Array::<felt252>
// ) -> Result::<PreparedContract, felt252> nopanic {
//     match prepare_impl(class_hash, calldata) {
//         Result::Ok((
//             constructor_calldata, contract_address, class_hash
//         )) => Result::<PreparedContract, felt252>::Ok(
//             PreparedContract {
//                 constructor_calldata: @constructor_calldata,
//                 contract_address: contract_address,
//                 class_hash: class_hash,
//                 }
//             ),
//         Result::Err(x) => Result::<PreparedContract, felt252>::Err(x)
//     }
// }

// extern fn prepare_impl(
//     class_hash: felt252, calldata: @Array::<felt252>
// ) -> Result::<(Array::<felt252>, felt252, felt252), felt252> nopanic;

// extern fn mock_call(
//     contract_address: felt252, function_name: felt252, response: @Array::<felt252>
// ) -> Result::<(), felt252> nopanic;

// extern fn start_roll(
//     block_number: felt252, target_contract_address: felt252
// ) -> Result::<(), felt252> nopanic;

// extern fn stop_roll(target_contract_address: felt252) -> Result::<(), felt252> nopanic;

// extern fn start_warp(
//     block_timestamp: felt252, target_contract_address: felt252
// ) -> Result::<(), felt252> nopanic;

// extern fn stop_warp(target_contract_address: felt252) -> Result::<(), felt252> nopanic;

// extern fn start_prank(
//     caller_address: felt252, target_contract_address: felt252
// ) -> Result::<(), felt252> nopanic;

// extern fn stop_prank(target_contract_address: felt252) -> Result::<(), felt252> nopanic;