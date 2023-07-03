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
    constructor_calldata: Span::<felt252>,
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
    let PreparedContract{contract_address, class_hash, mut constructor_calldata } =
        prepared_contract;
    let mut inputs = array![contract_address, class_hash];

    let calldata_len = constructor_calldata.len().into();
    inputs.append(calldata_len);

    loop {
        match constructor_calldata.pop_front() {
            Option::Some(value) => {
                inputs.append(*value);
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

        let mut i = 2;
        loop {
            if panic_data_len + 2 == i {
                break ();
            }
            let x = *outputs[i];
            panic_data.append(0);
            i += 1;
        };

        Result::<felt252, RevertedTransaction>::Err(RevertedTransaction { panic_data })
    }
}
