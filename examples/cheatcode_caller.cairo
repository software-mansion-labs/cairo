use result::ResultTrait;
use array::ArrayTrait;

const CONSTANT: felt = 1;

fn test_cheatcode_caller() {
    roll(CONSTANT, 2)
}

fn test_cheatcode_caller_twice() {
    roll(1, 2);
    roll(1, 2)
}

fn test_cheatcode_caller_three() {
    roll(1, 2);
    roll(1, 2);
    roll(1, 2)
}

fn test_declare() {
    match declare('test') {
        Result::Ok(class_hash) => (),
        Result::Err(x) => {
            let mut data = array_new::<felt>();
            array_append::<felt>(ref data, x);
            panic(data)
        },
    }
}

fn test_prepare() {
    let mut arr = ArrayTrait::new();
    arr.append(0xBAD);
    arr.append(0xC0DE);
    match prepare(0xBEEF, arr) {
        Result::Ok(prepared_contract) => {
            drop(prepared_contract)
        },
        Result::Err(x) => {
            let mut data = array_new::<felt>();
            array_append::<felt>(ref data, x);
            panic(data)
        },
    }
}

fn test_start_prank() {
    match start_prank(123, 123) {
        Result::Ok(class_hash) => (),
        Result::Err(x) => {
            let mut data = array_new::<felt>();
            array_append::<felt>(ref data, x);
            panic(data)
        },
    }
}

