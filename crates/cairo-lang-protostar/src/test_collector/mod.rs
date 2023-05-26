use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use cairo_felt::Felt252;
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_compiler::diagnostics::DiagnosticsReporter;
use cairo_lang_compiler::project::setup_project;
use cairo_lang_debug::DebugWithDb;
use cairo_lang_defs::ids::{FreeFunctionId, FunctionWithBodyId, ModuleItemId};
use cairo_lang_defs::plugin::PluginDiagnostic;
use cairo_lang_diagnostics::ToOption;
use cairo_lang_filesystem::cfg::{Cfg, CfgSet};
use cairo_lang_filesystem::ids::CrateId;
use cairo_lang_lowering::ids::ConcreteFunctionWithBodyId;
use cairo_lang_runner::Arg::Array;
use cairo_lang_semantic::db::SemanticGroup;
use cairo_lang_semantic::items::functions::GenericFunctionId;
use cairo_lang_semantic::{ConcreteFunction, FunctionLongId};
use cairo_lang_sierra::extensions::enm::EnumType;
use cairo_lang_sierra::extensions::NamedType;
use cairo_lang_sierra::program::{GenericArg, Program};
use cairo_lang_sierra_generator::db::SierraGenGroup;
use cairo_lang_sierra_generator::replace_ids::replace_sierra_ids_in_program;
use cairo_lang_starknet::plugin::StarkNetPlugin;
use cairo_lang_syntax::attribute::structured::{Attribute, AttributeArg, AttributeArgVariant};
use cairo_lang_syntax::node::ast;
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_test_runner::plugin::TestPlugin;
use cairo_lang_utils::OptionHelper;
use itertools::Itertools;
use num_traits::ToPrimitive;
use walkdir::WalkDir;

use crate::casm_generator::{SierraCasmGenerator, TestConfig as TestConfigInternal};
use crate::setup_project_without_cairo_project_toml;

/// Expectation for a panic case.
#[derive(Debug)]
pub enum PanicExpectation {
    /// Accept any panic value.
    Any,
    /// Accept only this specific vector of panics.
    Exact(Vec<Felt252>),
}

/// Expectation for a result of a test.
#[derive(Debug)]
pub enum TestExpectation {
    /// Running the test should not panic.
    Success,
    /// Running the test should result in a panic.
    Panics(PanicExpectation),
}

/// The configuration for running a single test.
#[derive(Debug)]
pub struct TestConfig {
    /// The amount of gas the test requested.
    pub available_gas: Option<usize>,
    /// The expected result of the run.
    pub expectation: TestExpectation,
    /// Should the test be ignored.
    pub ignored: bool,
}

/// Finds the tests in the requested crates.
pub fn find_all_tests(
    db: &dyn SemanticGroup,
    main_crates: Vec<CrateId>,
) -> Vec<(FreeFunctionId, TestConfig)> {
    let mut tests = vec![];
    for crate_id in main_crates {
        let modules = db.crate_modules(crate_id);
        for module_id in modules.iter() {
            let Ok(module_items) = db.module_items(*module_id) else {
                continue;
            };
            tests.extend(
                module_items.iter().filter_map(|item| {
                    let ModuleItemId::FreeFunction(func_id) = item else { return None };
                    let Ok(attrs) = db.function_with_body_attributes(FunctionWithBodyId::Free(*func_id)) else { return None };
                    Some((*func_id, try_extract_test_config(db.upcast(), attrs).unwrap()?))
                }),
            );
        }
    }
    tests
}

/// Extracts the configuration of a tests from attributes, or returns the diagnostics if the
/// attributes are set illegally.
pub fn try_extract_test_config(
    db: &dyn SyntaxGroup,
    attrs: Vec<Attribute>,
) -> Result<Option<TestConfig>, Vec<PluginDiagnostic>> {
    let test_attr = attrs.iter().find(|attr| attr.id.as_str() == "test");
    let ignore_attr = attrs.iter().find(|attr| attr.id.as_str() == "ignore");
    let available_gas_attr = attrs.iter().find(|attr| attr.id.as_str() == "available_gas");
    let should_panic_attr = attrs.iter().find(|attr| attr.id.as_str() == "should_panic");
    let mut diagnostics = vec![];
    if let Some(attr) = test_attr {
        if !attr.args.is_empty() {
            diagnostics.push(PluginDiagnostic {
                stable_ptr: attr.id_stable_ptr.untyped(),
                message: "Attribute should not have arguments.".into(),
            });
        }
    } else {
        for attr in [ignore_attr, available_gas_attr, should_panic_attr].into_iter().flatten() {
            diagnostics.push(PluginDiagnostic {
                stable_ptr: attr.id_stable_ptr.untyped(),
                message: "Attribute should only appear on tests.".into(),
            });
        }
    }
    let ignored = if let Some(attr) = ignore_attr {
        if !attr.args.is_empty() {
            diagnostics.push(PluginDiagnostic {
                stable_ptr: attr.id_stable_ptr.untyped(),
                message: "Attribute should not have arguments.".into(),
            });
        }
        true
    } else {
        false
    };
    let available_gas = if let Some(attr) = available_gas_attr {
        if let [
            AttributeArg {
                variant: AttributeArgVariant::Unnamed { value: ast::Expr::Literal(literal), .. },
                ..
            },
        ] = &attr.args[..]
        {
            literal.numeric_value(db).unwrap_or_default().to_usize()
        } else {
            diagnostics.push(PluginDiagnostic {
                stable_ptr: attr.id_stable_ptr.untyped(),
                message: "Attribute should have a single value argument.".into(),
            });
            None
        }
    } else {
        None
    };
    let (should_panic, expected_panic_value) = if let Some(attr) = should_panic_attr {
        if attr.args.is_empty() {
            (true, None)
        } else {
            (
                true,
                extract_panic_values(db, attr).on_none(|| {
                    diagnostics.push(PluginDiagnostic {
                        stable_ptr: attr.args_stable_ptr.untyped(),
                        message: "Expected panic must be of the form `expected = <tuple of \
                                  felts>`."
                            .into(),
                    });
                }),
            )
        }
    } else {
        (false, None)
    };
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    Ok(if test_attr.is_none() {
        None
    } else {
        Some(TestConfig {
            available_gas,
            expectation: if should_panic {
                TestExpectation::Panics(if let Some(values) = expected_panic_value {
                    PanicExpectation::Exact(values)
                } else {
                    PanicExpectation::Any
                })
            } else {
                TestExpectation::Success
            },
            ignored,
        })
    })
}

/// Tries to extract the relevant expected panic values.
fn extract_panic_values(db: &dyn SyntaxGroup, attr: &Attribute) -> Option<Vec<Felt252>> {
    let [
        AttributeArg {
            variant: AttributeArgVariant::Named { name, value: panics, .. },
            ..
        }
    ] = &attr.args[..] else {
        return None;
    };
    if name != "expected" {
        return None;
    }
    let ast::Expr::Tuple(panics) = panics else { return None };
    panics
        .expressions(db)
        .elements(db)
        .into_iter()
        .map(|value| match value {
            ast::Expr::Literal(literal) => {
                Some(literal.numeric_value(db).unwrap_or_default().into())
            }
            ast::Expr::ShortString(literal) => {
                Some(literal.numeric_value(db).unwrap_or_default().into())
            }
            _ => None,
        })
        .collect::<Option<Vec<_>>>()
}

// TODO docs
#[derive(Debug)]
pub struct LinkedLibrary {
    pub name: String,
    pub path: PathBuf,
}

// returns tuple[sierra if no output_path, list[test_name, test_config]]
pub fn collect_tests(
    input_path: &str,
    output_path: Option<&str>,
    linked_libraries: Option<Vec<LinkedLibrary>>,
    builtins: Option<Vec<&str>>,
) -> Result<(Program, Vec<TestConfigInternal>)> {
    // code taken from crates/cairo-lang-test-runner/src/lib.rs
    let db = &mut {
        let mut b = RootDatabase::builder();
        b.detect_corelib();
        b.with_cfg(CfgSet::from_iter([Cfg::name("test")]));
        b.with_semantic_plugin(Arc::new(TestPlugin::default()));
        b.with_semantic_plugin(Arc::new(StarkNetPlugin::default()));
        b.build()?
    };

    let mut entries: Vec<PathBuf> = vec![];
    for entry in WalkDir::new(&input_path) {
        if entry.is_err() {
            continue;
        }

        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_file() && path.extension().map_or(false, |ex| ex == "cairo") {
            entries.push(path.into());
        }
    }
    dbg!(&entries);

    let mut all_crate_ids: Vec<Vec<CrateId>> = vec![];
    for entry in entries {
        let crate_ids = setup_project(db, &entry)?;
        all_crate_ids.push(crate_ids);
    }

    // let main_crate_ids = setup_project(db, Path::new(&input_path))
    //     .with_context(|| format!("Failed to setup project for path({})", input_path))?;

    if let Some(linked_libraries) = linked_libraries {
        for linked_library in linked_libraries {
            setup_project_without_cairo_project_toml(
                db,
                &linked_library.path,
                &linked_library.name,
            )
            .with_context(|| format!("Failed to add linked library ({})", input_path))?;
        }
    }

    if DiagnosticsReporter::stderr().check(db) {
        return Err(anyhow!(
            "Failed to add linked library, for a detailed information, please go through the logs \
             above"
        ));
    }
    // let all_tests = find_all_tests(db, main_crate_ids);
    let mut all_tests: Vec<(FreeFunctionId, TestConfig)> = vec![];
    for crate_ids in all_crate_ids {
        let tests = find_all_tests(db, crate_ids);
        all_tests.extend(tests)
    }

    dbg!(&all_tests);

    let z: Vec<ConcreteFunctionWithBodyId> = all_tests
        .iter()
        .flat_map(|(func_id, _cfg)| ConcreteFunctionWithBodyId::from_no_generics_free(db, *func_id))
        .collect();

    let sierra_program = db
        .get_sierra_program_for_functions(z)
        .to_option()
        .context("Compilation failed without any diagnostics")
        .context("Failed to get sierra program")?;

    let collected_tests: Vec<TestConfigInternal> = all_tests
        .into_iter()
        .map(|(func_id, test)| {
            (
                format!(
                    "{:?}",
                    FunctionLongId {
                        function: ConcreteFunction {
                            generic_function: GenericFunctionId::Free(func_id),
                            generic_args: vec![]
                        }
                    }
                    .debug(db)
                ),
                test,
            )
        })
        .collect_vec()
        .into_iter()
        .map(|(test_name, config)| TestConfigInternal {
            name: test_name,
            available_gas: config.available_gas,
        })
        .collect();

    let sierra_program = replace_sierra_ids_in_program(db, &sierra_program);

    let mut builtins2 = vec![];
    if let Some(unwrapped_builtins) = builtins {
        builtins2 = unwrapped_builtins.iter().map(|s| s.to_string()).collect();
    }

    validate_tests(sierra_program.clone(), &collected_tests, builtins2)
        .context("Test validation failed")?;

    if let Some(path) = output_path {
        fs::write(path, &sierra_program.to_string()).context("Failed to write output")?;
    }
    Ok((sierra_program, collected_tests))
}

fn validate_tests(
    sierra_program: Program,
    collected_tests: &Vec<TestConfigInternal>,
    ignored_params: Vec<String>,
) -> Result<(), anyhow::Error> {
    let casm_generator = match SierraCasmGenerator::new(sierra_program) {
        Ok(casm_generator) => casm_generator,
        Err(e) => panic!("{}", e),
    };
    for test in collected_tests {
        let func = casm_generator.find_function(&test.name)?;
        let mut filtered_params: Vec<String> = Vec::new();
        for param in &func.params {
            let param_str = &param.ty.debug_name.as_ref().unwrap().to_string();
            if !ignored_params.contains(&param_str) {
                filtered_params.push(param_str.to_string());
            }
        }
        if !filtered_params.is_empty() {
            anyhow::bail!(format!(
                "Invalid number of parameters for test {}: expected 0, got {}",
                test.name,
                func.params.len()
            ));
        }
        let signature = &func.signature;
        let ret_types = &signature.ret_types;
        let tp = &ret_types[ret_types.len() - 1];
        let info = casm_generator.get_info(&tp);
        let mut maybe_return_type_name = None;
        if info.long_id.generic_id == EnumType::ID {
            if let GenericArg::UserType(ut) = &info.long_id.generic_args[0] {
                if let Some(name) = ut.debug_name.as_ref() {
                    maybe_return_type_name = Some(name.as_str());
                }
            }
        }
        if let Some(return_type_name) = maybe_return_type_name {
            if !return_type_name.starts_with("core::PanicResult::") {
                anyhow::bail!("Test function {} must be panicable but it's not", test.name);
            }
            if return_type_name != "core::PanicResult::<((),)>" {
                anyhow::bail!(
                    "Test function {} returns a value {}, it is required that test functions do \
                     not return values",
                    test.name,
                    return_type_name
                );
            }
        } else {
            anyhow::bail!(
                "Couldn't read result type for test function {} possible cause: Test function {} \
                 must be panicable but it's not",
                test.name,
                test.name
            );
        }
    }

    Ok(())
}
