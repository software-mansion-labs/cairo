const CONSTANT: felt = 1;

#[test]
fn test_cheatcode_caller() {
   roll(CONSTANT, 2)
}

#[test]
fn test_cheatcode_caller_twice() {
   roll(1, 2);
   roll(1, 2)
}

#[test]
fn test_cheatcode_caller_three() {
   roll(1, 2);
   roll(1, 2);
   roll(1, 2)
}

#[test]
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

#[test]
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

#[test]
#[should_panic]
fn test_start_prank_panic_no_caller_address() {
   match start_prank(0, 123) {
      Result::Ok(class_hash) => (),
      Result::Err(x) => {
         let mut data = array_new::<felt>();
         array_append::<felt>(ref data, x);
         panic(data)
      },
   }
}

#[test]
#[should_panic]
fn test_start_prank_panic_no_target_contract_address() {
   match start_prank(123, 0) {
      Result::Ok(class_hash) => (),
      Result::Err(x) => {
         let mut data = array_new::<felt>();
         array_append::<felt>(ref data, x);
         panic(data)
      },
   }
}
