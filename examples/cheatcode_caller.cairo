use result::ResultTrait;
use array::ArrayTrait;


fn test_roll() {
    match roll(1, 2) {
        Result::Ok(_) => (),
        Result::Err(x) => {
            let mut data = array_new::<felt>();
            array_append::<felt>(ref data, x);
            panic(data)
        },
    }
}

fn test_warp() {
    match warp(1, 2) {
        Result::Ok(_) => (),
        Result::Err(x) => {
            let mut data = array_new::<felt>();
            array_append::<felt>(ref data, x);
            panic(data)
        },
    }
}

fn test_declare() {
    match declare('test') {
        Result::Ok(_) => (),
        Result::Err(x) => {
            let mut data = array_new::<felt>();
            array_append::<felt>(ref data, x);
            panic(data)
        },
    }
}

fn test_start_prank() {
    match start_prank(123, 123) {
        Result::Ok(_) => (),
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

fn test_invoke() {
    let mut arr = ArrayTrait::new();
    arr.append(10);
    arr.append(11);
    arr.append(12);
    match invoke(123, 'test', arr) {
        Result::Ok(class_hash) => (),
        Result::Err(x) => {
            let mut data = array_new::<felt>();
            array_append::<felt>(ref data, x);
            panic(data)
        },
    }
}

fn test_mock_call() {
    let mut arr = ArrayTrait::new();
    arr.append(10);
    arr.append(11);
    arr.append(12);
    match mock_call(123, 'test', arr) {
        Result::Ok(()) => (),
        Result::Err(x) => {
            let mut data = array_new::<felt>();
            array_append::<felt>(ref data, x);
            panic(data)
        },
    }
}
