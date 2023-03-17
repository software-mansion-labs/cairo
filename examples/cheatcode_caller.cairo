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

fn test_declare_legacy() {
    match declare_legacy('test') {
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

fn test_stop_prank() {
    match stop_prank(123) {
        Result::Ok(class_hash) => (),
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

fn test_deploy() {
    let mut arr = ArrayTrait::new();
    arr.append(1);
    arr.append(2);
    match deploy(123, 123, arr) {
        Result::Ok(deployed_contract_address) => (),
        Result::Err(x) => {
            let mut data = array_new::<felt>();
            array_append::<felt>(ref data, x);
            panic(data)
        },
    }
}

fn test_deploy_wrapper() {
    let mut arr = ArrayTrait::new();
    arr.append(1);
    arr.append(2);
    arr.append(3);
    match deploy_wrapper(
        PreparedContract { address: 123, class_hash: 123, constructor_calldata: arr }
    ) {
        Result::Ok(deployed_contract_address) => (),
        Result::Err(x) => {
            let mut data = array_new::<felt>();
            array_append::<felt>(ref data, x);
            panic(data)
        },
    }
}
