use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use cairo_lang_compiler::CompilerConfig;
use cairo_lang_protostar::casm_generator::TestConfig;
use cairo_lang_protostar::test_collector::{
    collect_tests as internal_collect_tests, LinkedLibrary,
};
use cairo_lang_protostar::{build_protostar_casm_from_sierra, compile_from_resolved_dependencies};
use cairo_lang_starknet::casm_contract_class::CasmContractClass;
use cairo_lang_starknet::contract_class::ContractClass;
use pyo3::exceptions::RuntimeError;
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

type CollectedTest = (String, Option<usize>);

fn ensure_path_is_dir(path_str: &str) -> Result<(), anyhow::Error> {
    let path = Path::new(path_str);
    if !path.is_dir() {
        let mut err_msg = "invalid input path: a directory path is expected".to_owned();
        if path.is_file() {
            err_msg.push_str(", a file was received");
        }
        anyhow::bail!(err_msg);
    }
    Ok(())
}

#[pyfunction]
fn compile_starknet_contract_to_sierra_from_path(
    input_path: &str,
    output_path: Option<&str>,
    maybe_cairo_paths: Option<Vec<(&str, &str)>>,
) -> PyResult<String> {
    ensure_path_is_dir(input_path)
        .map_err(|e| PyErr::new::<RuntimeError, _>(format!("{:?}", e)))?;
    let sierra = starknet_cairo_to_sierra(input_path, maybe_cairo_paths)
        .map_err(|e| PyErr::new::<RuntimeError, _>(format!("{:?}", e)))?;

    if let Some(path) = output_path {
        fs::write(path, &sierra).map_err(|e| {
            PyErr::new::<RuntimeError, _>(format!("Failed to write output: {:?}", e))
        })?;
    }
    Ok(sierra)
}

fn starknet_cairo_to_sierra(
    input_path: &str,
    maybe_cairo_paths: Option<Vec<(&str, &str)>>,
) -> Result<String, anyhow::Error> {
    let contract = compile_from_resolved_dependencies(
        input_path,
        None,
        CompilerConfig { replace_ids: true, ..CompilerConfig::default() },
        maybe_cairo_paths,
    )?;
    let sierra =
        serde_json::to_string_pretty(&contract).with_context(|| "Serialization failed.")?;

    Ok(sierra)
}

#[pyfunction]
fn compile_starknet_contract_to_casm_from_path(
    input_path: &str,
    output_path: Option<&str>,
    maybe_cairo_paths: Option<Vec<(&str, &str)>>,
) -> PyResult<String> {
    ensure_path_is_dir(input_path)
        .map_err(|e| PyErr::new::<RuntimeError, _>(format!("{:?}", e)))?;
    let casm = starknet_cairo_to_casm(input_path, maybe_cairo_paths)
        .map_err(|e| PyErr::new::<RuntimeError, _>(format!("{:?}", e)))?;

    if let Some(path) = output_path {
        fs::write(path, &casm).map_err(|e| {
            PyErr::new::<RuntimeError, _>(format!("Failed to write output: {:?}", e))
        })?;
    }
    Ok(casm)
}

fn starknet_sierra_to_casm(sierra: &str) -> Result<String, anyhow::Error> {
    let contract_class: ContractClass =
        serde_json::from_str(&sierra[..]).with_context(|| "deserialization Failed.")?;

    let casm_contract = CasmContractClass::from_contract_class(contract_class, true)
        .with_context(|| "Compilation failed.")?;

    let casm =
        serde_json::to_string_pretty(&casm_contract).with_context(|| "Serialization failed.")?;

    Ok(casm)
}

fn starknet_cairo_to_casm(
    input_path: &str,
    maybe_cairo_paths: Option<Vec<(&str, &str)>>,
) -> Result<String, anyhow::Error> {
    let sierra = starknet_cairo_to_sierra(input_path, maybe_cairo_paths)?;
    starknet_sierra_to_casm(&sierra)
}

#[pyfunction]
fn compile_starknet_contract_sierra_to_casm_from_path(
    input_path: &str,
    output_path: Option<&str>,
) -> PyResult<String> {
    let sierra = fs::read_to_string(input_path).expect("Could not read file!");
    compile_starknet_contract_sierra_to_casm_from_sierra_code(&sierra, output_path)
}

#[pyfunction]
fn compile_starknet_contract_sierra_to_casm_from_sierra_code(
    sierra_compiled: &str,
    output_path: Option<&str>,
) -> PyResult<String> {
    let casm = starknet_sierra_to_casm(sierra_compiled)
        .map_err(|e| PyErr::new::<RuntimeError, _>(format!("{:?}", e)))?;

    if let Some(path) = output_path {
        fs::write(path, &casm).map_err(|e| {
            PyErr::new::<RuntimeError, _>(format!("Failed to write output: {:?}", e))
        })?;
    }
    Ok(casm)
}

fn build_linked_libraries(cairo_paths: Vec<(&str, &str)>) -> Vec<LinkedLibrary> {
    cairo_paths
        .into_iter()
        .map(|(path, name)| LinkedLibrary { name: name.to_string(), path: PathBuf::from(path) })
        .collect()
}

// returns tuple[sierra, list[test_name, test_config]]
#[pyfunction]
fn collect_tests(
    input_path: &str,
    output_path: Option<&str>,
    maybe_cairo_paths: Option<Vec<(&str, &str)>>,
    maybe_builtins: Option<Vec<&str>>,
) -> PyResult<(String, Vec<CollectedTest>)> {
    let linked_libraries = maybe_cairo_paths.map(build_linked_libraries);

    let (sierra_program, collected) = internal_collect_tests(
        input_path,
        output_path,
        linked_libraries,
        maybe_builtins.as_ref().map(|v| v.iter().map(|&s| s).collect()),
        None,
    )
    .map_err(|e| {
        PyErr::new::<RuntimeError, _>(format!(
            "Failed to setup project for path({}): {:?}",
            input_path, e
        ))
    })?;
    let external_collected = collected.iter().map(|c| (c.name.clone(), c.available_gas)).collect();

    Ok((sierra_program.to_string(), external_collected))
}

#[pyfunction]
fn compile_protostar_sierra_to_casm(
    collected_tests: Vec<CollectedTest>,
    input_data: String,
    output_path: Option<&str>,
) -> PyResult<Option<String>> {
    let internal_collected = collected_tests
        .iter()
        .map(|c| TestConfig { name: c.0.clone(), available_gas: c.1.clone() })
        .collect();
    let casm = build_protostar_casm_from_sierra(
        &internal_collected,
        input_data,
        output_path.map(|s| s.to_string()),
    )
    .map_err(|e| PyErr::new::<RuntimeError, _>(format!("{:?}", e)))?;
    Ok(casm)
}

#[pyfunction]
fn compile_protostar_sierra_to_casm_from_path(
    collected_tests: Vec<CollectedTest>,
    input_path: &str,
    output_path: Option<&str>,
) -> PyResult<Option<String>> {
    let input_data = fs::read_to_string(input_path).expect("Could not read file!");
    let internal_collected = collected_tests
        .iter()
        .map(|c| TestConfig { name: c.0.clone(), available_gas: c.1.clone() })
        .collect();
    let casm = build_protostar_casm_from_sierra(
        &internal_collected,
        input_data,
        output_path.map(|s| s.to_string()),
    )
    .map_err(|e| PyErr::new::<RuntimeError, _>(format!("{:?}", e)))?;

    Ok(casm)
}

#[pymodule]
fn cairo_python_bindings(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pyfunction!(compile_starknet_contract_to_casm_from_path))?;
    m.add_wrapped(wrap_pyfunction!(compile_starknet_contract_to_sierra_from_path))?;
    m.add_wrapped(wrap_pyfunction!(compile_starknet_contract_sierra_to_casm_from_path))?;
    m.add_wrapped(wrap_pyfunction!(compile_starknet_contract_sierra_to_casm_from_sierra_code))?;
    m.add_wrapped(wrap_pyfunction!(collect_tests))?;
    m.add_wrapped(wrap_pyfunction!(compile_protostar_sierra_to_casm))?;
    m.add_wrapped(wrap_pyfunction!(compile_protostar_sierra_to_casm_from_path))?;
    Ok(())
}
