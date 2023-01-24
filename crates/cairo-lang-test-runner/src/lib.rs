
use std::sync::Mutex;

use anyhow::Context;
use cairo_lang_defs::ids::{FreeFunctionId, FunctionWithBodyId, ModuleItemId};
use cairo_lang_filesystem::ids::CrateId;
use cairo_lang_runner::{RunResultValue, SierraCasmRunner};
use cairo_lang_semantic::db::SemanticGroup;
use cairo_lang_syntax::node::ast::Expr;
use cairo_lang_syntax::node::Token;
use colored::Colorize;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};


use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_compiler::diagnostics::check_and_eprint_diagnostics;
use cairo_lang_compiler::project::setup_project;
use cairo_lang_diagnostics::ToOption;
use cairo_lang_plugins::config::ConfigPlugin;
use cairo_lang_plugins::derive::DerivePlugin;
use cairo_lang_plugins::panicable::PanicablePlugin;
use cairo_lang_semantic::plugin::SemanticPlugin;
use cairo_lang_sierra_generator::db::SierraGenGroup;
use cairo_lang_sierra_generator::replace_ids::replace_sierra_ids_in_program;
use cairo_lang_starknet::plugin::StarkNetPlugin;
use clap::Parser;

/// The status of a ran test.
enum TestStatus {
  Success,
  Fail(RunResultValue),
  Ignore,
}

/// Summary data of the ran tests.
pub struct TestsSummary {
  pub passed: Vec<String>,
  pub failed: Vec<String>,
  pub ignored: Vec<String>,
  pub failed_run_results: Vec<RunResultValue>,
}

/// Command line args parser.
/// Exits with 0/1 if the input is formatted correctly/incorrectly.
#[derive(Parser, Debug)]
#[clap(version, verbatim_doc_comment)]
pub struct Args {
    /// The path to compile and run its tests.
    #[arg(short, long)]
    pub path: String,
    /// The filter for the tests, running only tests containing the filter string.
    #[arg(short, long, default_value_t = String::default())]
    pub filter: String,
    /// Should we run ignored tests as well.
    #[arg(long, default_value_t = false)]
    pub include_ignored: bool,
    /// Should we run only the ignored tests.
    #[arg(long, default_value_t = false)]
    pub ignored: bool,
    /// Should we add the starknet plugin to run the tests.
    #[arg(long, default_value_t = false)]
    pub starknet: bool,
}

pub fn get_db(maybe_args: Option<&Args>) -> RootDatabase {
    // TODO(orizi): Use `get_default_plugins` and just update the config plugin.
    let mut plugins: Vec<Arc<dyn SemanticPlugin>> = vec![
        Arc::new(DerivePlugin {}),
        Arc::new(PanicablePlugin {}),
        Arc::new(ConfigPlugin { configs: HashSet::from(["test".to_string()]) }),
    ];
    if let Some(args) = maybe_args {
        if args.starknet {
            plugins.push(Arc::new(StarkNetPlugin {}));
        }
    }
    return RootDatabase::new(plugins);
}

pub fn find_all_tests_wrapper(path: &str, db: &mut RootDatabase) -> Result<Vec<TestConfig>, anyhow::Error> {
    let main_crate_ids = setup_project(db, Path::new(path))?;
    println!(">>>>>>>>>>>>> PATH: {}", path);

    if check_and_eprint_diagnostics(db) {
        anyhow::bail!("failed to compile: {}", path);
    }
    Ok(find_all_tests(db, main_crate_ids))
}

pub fn get_sierra_program(all_tests: &Vec<TestConfig>, db: &mut RootDatabase) -> Result<cairo_lang_sierra::program::Program, anyhow::Error> {
    let sierra_program = db
        .get_sierra_program_for_functions(
            all_tests.iter().map(|t| FunctionWithBodyId::Free(t.func_id)).collect(),
        )
        .to_option()
        .with_context(|| "Compilation failed without any diagnostics.")?;
    Ok(replace_sierra_ids_in_program(db, &sierra_program))
}

/// Runs the tests and process the results for a summary.
pub fn run_tests(
  named_tests: Vec<(String, TestConfig)>,
  sierra_program: cairo_lang_sierra::program::Program,
) -> anyhow::Result<TestsSummary> {
  let runner =
      SierraCasmRunner::new(sierra_program, true).with_context(|| "Failed setting up runner.")?;
  println!("running {} tests", named_tests.len());
  let wrapped_summary = Mutex::new(Ok(TestsSummary {
      passed: vec![],
      failed: vec![],
      ignored: vec![],
      failed_run_results: vec![],
  }));
  named_tests
      .into_par_iter()
      .map(|(name, test)| -> anyhow::Result<(String, TestStatus)> {
          if test.ignored {
              return Ok((name, TestStatus::Ignore));
          }
          let result = runner
              .run_function(name.as_str(), &[], test.available_gas)
              .with_context(|| "Failed to run the function.")?;
          Ok((
              name,
              match (&result.value, test.expectation) {
                  (RunResultValue::Success(_), TestExpectation::Success)
                  | (RunResultValue::Panic(_), TestExpectation::Panics) => TestStatus::Success,
                  (RunResultValue::Success(_), TestExpectation::Panics)
                  | (RunResultValue::Panic(_), TestExpectation::Success) => {
                      TestStatus::Fail(result.value)
                  }
              },
          ))
      })
      .for_each(|r| {
          let mut wrapped_summary = wrapped_summary.lock().unwrap();
          if wrapped_summary.is_err() {
              return;
          }
          let (name, status) = match r {
              Ok((name, status)) => (name, status),
              Err(err) => {
                  *wrapped_summary = Err(err);
                  return;
              }
          };
          let summary = wrapped_summary.as_mut().unwrap();
          let (res_type, status_str) = match status {
              TestStatus::Success => (&mut summary.passed, "ok".bright_green()),
              TestStatus::Fail(run_result) => {
                  summary.failed_run_results.push(run_result);
                  (&mut summary.failed, "fail".bright_red())
              }
              TestStatus::Ignore => (&mut summary.ignored, "ignored".bright_yellow()),
          };
          println!("test {name} ... {status_str}",);
          res_type.push(name);
      });
  wrapped_summary.into_inner().unwrap()
}

/// Expectation for a result of a test.
pub enum TestExpectation {
  /// Running the test should not panic.
  Success,
  /// Running the test should result in a panic.
  Panics,
}

/// The configuration for running a single test.
pub struct TestConfig {
  /// The function id of the test function.
  pub func_id: FreeFunctionId,
  /// The amount of gas the test requested.
  pub available_gas: Option<usize>,
  /// The expected result of the run.
  pub expectation: TestExpectation,
  /// Should the test be ignored.
  pub ignored: bool,
}

/// Finds the tests in the requested crates.
pub fn find_all_tests(db: &dyn SemanticGroup, main_crates: Vec<CrateId>) -> Vec<TestConfig> {
  let mut tests = vec![];
  for crate_id in main_crates {
      let modules = db.crate_modules(crate_id);
      for module_id in modules.iter() {
          let Ok(module_items) = db.module_items(*module_id) else {
              continue;
          };

          for item in module_items.iter() {
              if let ModuleItemId::FreeFunction(func_id) = item {
                  if let Ok(attrs) =
                      db.function_with_body_attributes(FunctionWithBodyId::Free(*func_id))
                  {
                      let mut is_test = false;
                      let mut available_gas = None;
                      let mut ignored = false;
                      let mut should_panic = false;
                      for attr in attrs {
                          match attr.id.as_str() {
                              "test" => {
                                  is_test = true;
                              }
                              "available_gas" => {
                                  // TODO(orizi): Provide diagnostics when this does not match.
                                  if let [Expr::Literal(literal)] = &attr.args[..] {
                                      available_gas = literal
                                          .token(db.upcast())
                                          .text(db.upcast())
                                          .parse::<usize>()
                                          .ok();
                                  }
                              }
                              "should_panic" => {
                                  should_panic = true;
                              }
                              "ignore" => {
                                  ignored = true;
                              }
                              _ => {}
                          }
                      }
                      if is_test {
                          tests.push(TestConfig {
                              func_id: *func_id,
                              available_gas,
                              expectation: if should_panic {
                                  TestExpectation::Panics
                              } else {
                                  TestExpectation::Success
                              },
                              ignored,
                          })
                      }
                  }
              }
          }
      }
  }
  tests
}
