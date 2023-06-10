use array::ArrayTrait;
use option::OptionTrait;
use clone::Clone;
use traits::Into;
use integer::U128IntoFelt252;

extern fn start_roll(
    block_number: felt252, target_contract_address: felt252
) -> Result::<(), felt252> nopanic;

extern fn stop_roll(target_contract_address: felt252) -> Result::<(), felt252> nopanic;

extern fn start_warp(
    block_timestamp: felt252, target_contract_address: felt252
) -> Result::<(), felt252> nopanic;

extern fn stop_warp(target_contract_address: felt252) -> Result::<(), felt252> nopanic;

extern fn start_prank(
    caller_address: felt252, target_contract_address: felt252
) -> Result::<(), felt252> nopanic;

extern fn stop_prank(target_contract_address: felt252) -> Result::<(), felt252> nopanic;

extern fn declare(contract: felt252) -> Result::<felt252, felt252> nopanic;

extern fn declare_cairo0(contract: felt252) -> Result::<felt252, felt252> nopanic;

#[derive(Drop, Clone)]
struct RevertedTransaction {
    panic_data: Array::<felt252>, 
}

trait RevertedTransactionTrait {
    fn first(self: @RevertedTransaction) -> felt252;
}

impl RevertedTransactionImpl of RevertedTransactionTrait {
    fn first(self: @RevertedTransaction) -> felt252 {
        *self.panic_data.at(0_usize)
    }
}

extern fn invoke_impl(
    contract_address: felt252, function_name: felt252, calldata: @Array::<felt252>
) -> Result::<(), Array<felt252>> nopanic;

fn invoke(
    contract_address: felt252, function_name: felt252, calldata: @Array::<felt252>
) -> Result::<(), RevertedTransaction> nopanic {
    match invoke_impl(contract_address, function_name, calldata) {
        Result::Ok(x) => Result::<(), RevertedTransaction>::Ok(x),
        Result::Err(x) => Result::<(),
        RevertedTransaction>::Err(RevertedTransaction { panic_data: x,  })
    }
}

extern fn mock_call(
    contract_address: felt252, function_name: felt252, response: @Array::<felt252>
) -> Result::<(), felt252> nopanic;

#[derive(Drop, Clone)]
struct PreparedContract {
    contract_address: felt252,
    class_hash: felt252,
    constructor_calldata: @Array::<felt252>,
}

// returns deployed `contract_address`
extern fn deploy_impl(
    prepared_contract_address: felt252,
    prepared_class_hash: felt252,
    prepared_constructor_calldata: @Array::<felt252>
) -> Result::<felt252, Array<felt252>> nopanic;

fn deploy(prepared_contract: PreparedContract) -> Result::<felt252, RevertedTransaction> nopanic {
    let PreparedContract{contract_address, class_hash, constructor_calldata } = prepared_contract;
    match deploy_impl(contract_address, class_hash, constructor_calldata) {
        Result::Ok(x) => Result::<felt252, RevertedTransaction>::Ok(x),
        Result::Err(x) => Result::<felt252,
        RevertedTransaction>::Err(RevertedTransaction { panic_data: x,  })
    }
}

extern fn prepare_impl(
    class_hash: felt252, calldata: @Array::<felt252>
) -> Result::<(Array::<felt252>, felt252, felt252), felt252> nopanic;

fn prepare(
    class_hash: felt252, calldata: @Array::<felt252>
) -> Result::<PreparedContract, felt252> nopanic {
    match prepare_impl(class_hash, calldata) {
        Result::Ok((
            constructor_calldata, contract_address, class_hash
        )) => Result::<PreparedContract,
        felt252>::Ok(
            PreparedContract {
                constructor_calldata: @constructor_calldata,
                contract_address: contract_address,
                class_hash: class_hash,
            }
        ),
        Result::Err(x) => Result::<PreparedContract, felt252>::Err(x)
    }
}

fn deploy_contract(
    contract: felt252, calldata: @Array::<felt252>
) -> Result::<felt252, RevertedTransaction> {
    let mut class_hash: Option::<felt252> = Option::None(());
    match declare(contract) {
        Result::Ok(x) => {
            class_hash = Option::Some(x);
        },
        Result::Err(x) => {
            let mut panic_data = ArrayTrait::new();
            panic_data.append(x);

            return Result::<felt252, RevertedTransaction>::Err(RevertedTransaction { panic_data });
        }
    }

    let mut prepared_contract: Option::<PreparedContract> = Option::None(());
    match prepare(class_hash.unwrap(), calldata) {
        Result::Ok(x) => {
            prepared_contract = Option::Some(x);
        },
        Result::Err(x) => {
            let mut panic_data = ArrayTrait::new();
            panic_data.append(x);

            return Result::<felt252, RevertedTransaction>::Err(RevertedTransaction { panic_data });
        }
    }
    deploy(prepared_contract.unwrap())
}

fn deploy_contract_cairo0(
    contract: felt252, calldata: @Array::<felt252>
) -> Result::<felt252, RevertedTransaction> {
    let mut class_hash: Option::<felt252> = Option::None(());
    match declare_cairo0(contract) {
        Result::Ok(x) => {
            class_hash = Option::Some(x);
        },
        Result::Err(x) => {
            let mut panic_data = ArrayTrait::new();
            panic_data.append(x);

            return Result::<felt252, RevertedTransaction>::Err(RevertedTransaction { panic_data });
        }
    }

    let mut prepared_contract: Option::<PreparedContract> = Option::None(());
    match prepare(class_hash.unwrap(), calldata) {
        Result::Ok(x) => {
            prepared_contract = Option::Some(x);
        },
        Result::Err(x) => {
            let mut panic_data = ArrayTrait::new();
            panic_data.append(x);

            return Result::<felt252, RevertedTransaction>::Err(RevertedTransaction { panic_data });
        }
    }
    deploy(prepared_contract.unwrap())
}

extern fn call_impl(
    contract: felt252, function_name: felt252, calldata: @Array::<felt252>
) -> Result::<Array<felt252>, Array<felt252>> nopanic;


fn call(
    contract: felt252, function_name: felt252, calldata: @Array::<felt252>
) -> Result::<Array<felt252>, RevertedTransaction> nopanic {
    match call_impl(contract, function_name, calldata) {
        Result::Ok(x) => Result::<Array<felt252>, RevertedTransaction>::Ok(x),
        Result::Err(x) => Result::<Array<felt252>,
        RevertedTransaction>::Err(RevertedTransaction { panic_data: x,  })
    }
}

struct TxInfoMock {
    version: Option<felt252>,
    account_contract_address: Option<felt252>,
    max_fee: Option<u128>,
    signature: Option<Array<felt252>>,
    transaction_hash: Option<felt252>,
    chain_id: Option<felt252>,
    nonce: Option<felt252>,
}

trait TxInfoMockTrait {
    fn default() -> TxInfoMock;
}

impl TxInfoMockImpl of TxInfoMockTrait {
    fn default() -> TxInfoMock {
        TxInfoMock {
            version: Option::None(()),
            account_contract_address: Option::None(()),
            max_fee: Option::None(()),
            signature: Option::None(()),
            transaction_hash: Option::None(()),
            chain_id: Option::None(()),
            nonce: Option::None(()),
        }
    }
}

fn setter_and_value<T, impl TDrop: Drop::<T>>(option: Option<T>, default: T) -> (bool, T) {
    let is_set = option.is_some();
    let value = option.unwrap_or_else(default);
    (is_set, value)
}

fn start_spoof(contract_address: felt252, mock: TxInfoMock) {
    let TxInfoMock{version, account_contract_address, max_fee, signature, transaction_hash, chain_id, nonce} = mock;

    let (set_version, version) = setter_and_value(version, 0);

    let (set_account_contract_address, account_contract_address) = setter_and_value(account_contract_address, 0);

    let (set_max_fee, max_fee) = setter_and_value(max_fee, 0_u128);

    let (set_signature, signature) = setter_and_value(signature, ArrayTrait::new());

    let (set_transaction_hash, transaction_hash) = setter_and_value(transaction_hash, 0);

    let (set_chain_id, chain_id) = setter_and_value(chain_id, 0);

    let (set_nonce, nonce) = setter_and_value(nonce, 0);

    start_spoof_impl(
        contract_address: contract_address,
        version: version,
        set_version: set_version,
        account_contract_address: account_contract_address,
        set_account_contract_address: set_account_contract_address,
        max_fee: max_fee.into(),
        set_max_fee: set_max_fee,
        signature: signature,
        set_signature: set_signature,
        transaction_hash: transaction_hash,
        set_transaction_hash: set_transaction_hash,
        chain_id: chain_id,
        set_chain_id: set_chain_id,
        nonce: nonce,
        set_nonce: set_nonce
    )
}

extern fn start_spoof_impl(
    contract_address: felt252,
    version: felt252,
    set_version: bool,
    account_contract_address: felt252,
    set_account_contract_address: bool,
    max_fee: felt252,
    set_max_fee: bool,
    signature: Array::<felt252>,
    set_signature: bool,
    transaction_hash: felt252,
    set_transaction_hash: bool,
    chain_id: felt252,
    set_chain_id: bool,
    nonce: felt252,
    set_nonce: bool
) nopanic;

// extern fn stop_spoof(contract_address) nopanic;
