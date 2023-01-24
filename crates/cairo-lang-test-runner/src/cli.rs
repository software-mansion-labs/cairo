//! Compiles and runs a Cairo program.

use anyhow::bail;
use cairo_lang_debug::DebugWithDb;
use cairo_lang_defs::ids::FunctionWithBodyId;
use cairo_lang_runner::short_string::as_cairo_short_string;
use cairo_lang_runner::RunResultValue;
use cairo_lang_semantic::items::functions::GenericFunctionId;
use cairo_lang_semantic::{ConcreteFunction, FunctionLongId};
use colored::Colorize;
use itertools::Itertools;

use clap::Parser;

use cairo_lang_test_runner::{Args, run_tests, TestsSummary, get_db, find_all_tests_wrapper, get_sierra_program};

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let mut db_val = get_db(Some(&args));
    let db = &mut db_val;

    let all_tests = find_all_tests_wrapper(&args.path, db)?;

    println!(">>>>>>>>>>>>> ALL TESTS: {}", all_tests.len());
    for test in &all_tests {
        let func = FunctionWithBodyId::Free(test.func_id);
        println!(">>>>>>>>>>>>> TEST loop: {:?}/{:?}", test.func_id, func);
    }
    let sierra_program = get_sierra_program(&all_tests, db)?;
    // println!(">>>>>>>>>>>>> SIERRA PROGRAM: {}", sierra_program.to_string());
    let total_tests_count = all_tests.len();
    let named_tests = all_tests
        .into_iter()
        .map(|mut test| {
            // Un-ignoring all the tests in `include-ignored` mode.
            if args.include_ignored {
                test.ignored = false;
            }
            (
                format!(
                    "{:?}",
                    FunctionLongId {
                        function: ConcreteFunction {
                            generic_function: GenericFunctionId::Free(test.func_id),
                            generic_args: vec![]
                        }
                    }
                    .debug(db)
                ),
                test,
            )
        })
        .filter(|(name, _)| name.contains(&args.filter))
        // Filtering unignored tests in `ignored` mode.
        .filter(|(_, test)| !args.ignored || test.ignored)
        .collect_vec();
    let filtered_out = total_tests_count - named_tests.len();
    let TestsSummary { passed, failed, ignored, failed_run_results } =
        run_tests(named_tests, sierra_program)?;
    if failed.is_empty() {
        println!(
            "test result: {}. {} passed; {} failed; {} ignored; {filtered_out} filtered out;",
            "ok".bright_green(),
            passed.len(),
            failed.len(),
            ignored.len()
        );
        Ok(())
    } else {
        println!("failures:");
        for (failure, run_result) in failed.iter().zip_eq(failed_run_results) {
            print!("   {failure} - ");
            match run_result {
                RunResultValue::Success(_) => {
                    println!("expected panic but finished successfully.");
                }
                RunResultValue::Panic(values) => {
                    print!("panicked with [");
                    for value in &values {
                        match as_cairo_short_string(value) {
                            Some(as_string) => print!("{value} ('{as_string}'), "),
                            None => print!("{value}, "),
                        }
                    }
                    println!("].")
                }
            }
        }
        println!();
        bail!(
            "test result: {}. {} passed; {} failed; {} ignored",
            "FAILED".bright_red(),
            passed.len(),
            failed.len(),
            ignored.len()
        );
    }
}
