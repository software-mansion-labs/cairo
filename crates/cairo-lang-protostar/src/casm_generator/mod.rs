//! Basic runner for running a Sierra program on the vm.
use std::collections::HashMap;

use cairo_lang_casm::instructions::Instruction;
use cairo_lang_casm::{casm, casm_extend};
use cairo_lang_sierra::extensions::core::{CoreLibfunc, CoreType};
use cairo_lang_sierra::extensions::ConcreteType;
use cairo_lang_sierra::program::{Function};
use cairo_lang_sierra::program_registry::{ProgramRegistry, ProgramRegistryError};
use cairo_lang_sierra_ap_change::{calc_ap_changes, ApChangeError};
use cairo_lang_sierra_gas::calc_gas_info;
use cairo_lang_sierra_gas::gas_info::GasInfo;
use cairo_lang_sierra_to_casm::compiler::{CairoProgram, CompilationError};
use cairo_lang_sierra_to_casm::metadata::Metadata;
use cairo_lang_starknet::casm_contract_class::{serialize_big_uint, deserialize_big_uint, BigIntAsHex};
use cairo_vm::vm::errors::vm_errors::VirtualMachineError;
use itertools::chain;
use num_bigint::BigUint;
use num_traits::{Num, Signed};
use num_integer::Integer;
use serde::{Deserialize, Serialize};
use num_bigint::BigInt;
use thiserror::Error;   
use smol_str::SmolStr;


#[derive(Debug, Error)]
pub enum GeneratorError {
    #[error("Failed calculating gas usage, it is likely a call for `get_gas` is missing.")]
    FailedGasCalculation,
    #[error("Function with suffix `{suffix}` to run not found.")]
    MissingFunction { suffix: String },
    #[error("Function expects arguments of size {expected} and received {actual} instead.")]
    ArgumentsSizeMismatch { expected: usize, actual: usize },
    #[error(transparent)]
    ProgramRegistryError(#[from] Box<ProgramRegistryError>),
    #[error(transparent)]
    SierraCompilationError(#[from] CompilationError),
    #[error(transparent)]
    ApChangeError(#[from] ApChangeError),
    #[error(transparent)]
    VirtualMachineError(#[from] Box<VirtualMachineError>),
}


#[derive(Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestEntrypoint {
    pub offset: usize,
    pub name: String
}

#[derive(Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtostarCasm {
    #[serde(serialize_with = "serialize_big_uint", deserialize_with = "deserialize_big_uint")]
    pub prime: BigUint,
    pub bytecode: Vec<BigIntAsHex>,
    pub hints: Vec<(usize, Vec<String>)>,
    pub test_entry_points: Vec<TestEntrypoint>,
}

pub struct SierraCasmGenerator {
    /// The sierra program.
    sierra_program: cairo_lang_sierra::program::Program,
    /// Program registry for the Sierra program.
    sierra_program_registry: ProgramRegistry<CoreType, CoreLibfunc>,
    /// The casm program matching the Sierra code.
    casm_program: CairoProgram,
}

impl SierraCasmGenerator {
    pub fn new(
        sierra_program: cairo_lang_sierra::program::Program,
        calc_gas: bool,
    ) -> Result<Self, GeneratorError> {
        let metadata = create_metadata(&sierra_program, calc_gas)?;
        let sierra_program_registry =
            ProgramRegistry::<CoreType, CoreLibfunc>::new(&sierra_program)?;
        let casm_program =
            cairo_lang_sierra_to_casm::compiler::compile(&sierra_program, &metadata, calc_gas)?;
        Ok(Self { sierra_program, sierra_program_registry, casm_program })
    }
    
    pub fn collect_tests(&self) -> Vec<&SmolStr> {
        self.sierra_program
        .funcs
        .iter()
        .filter(|f|
            if let Some(name) = &f.id.debug_name { name.contains("test_") } else { false }
        )
        .map(|f| 
            match &f.id.debug_name {
                Some(name) => name,
                _ => panic!("Expected name")
            } 
        )
        .collect::<Vec<&SmolStr>>()
    }
    
    pub fn build_casm(&self) -> Result<ProtostarCasm, GeneratorError> {
        let tests = self.collect_tests();
        let mut entry_codes = Vec::new();
        for test in &tests {
            let func = self.find_function(test)?;
            let initial_gas = 0; 
            let (entry_code, _) = self.create_entry_code(func, &vec![], initial_gas)?;
            entry_codes.push(entry_code);
        }


        let footer = self.create_code_footer();
        let prime = BigUint::from_str_radix(
            "800000000000011000000000000000000000000000000000000000000000001",
            16,
        )
        .unwrap();

        let mut bytecode = vec![];
        let mut hints = vec![];
        
        for entry_code in &entry_codes {
            for instruction in entry_code {
                if !instruction.hints.is_empty() {
                    hints.push((
                        bytecode.len(),
                        instruction.hints.iter().map(|hint| hint.to_string()).collect(),
                    ))
                }
                bytecode.extend(instruction.assemble().encode().iter().map(|big_int| {
                    let (_q, reminder) = big_int.magnitude().div_rem(&prime);
    
                    BigIntAsHex {
                        value: if big_int.is_negative() { &prime - reminder } else { reminder },
                    }
                }))
            }
        }

        for instruction in chain!( self.casm_program.instructions.iter(), footer.iter()) {
            if !instruction.hints.is_empty() {
                hints.push((
                    bytecode.len(),
                    instruction.hints.iter().map(|hint| hint.to_string()).collect(),
                ))
            }
            bytecode.extend(instruction.assemble().encode().iter().map(|big_int| {
                let (_q, reminder) = big_int.magnitude().div_rem(&prime);

                BigIntAsHex {
                    value: if big_int.is_negative() { &prime - reminder } else { reminder },
                }
            }))
        }

        let mut test_entry_points = Vec::new();
        let mut acc = 0;
        for (test, entry_code) in tests.iter().zip(entry_codes.iter()) {
            test_entry_points.push(TestEntrypoint {
                offset: acc,
                name: test.to_string()
            });
            acc += entry_code.len();
        }
        Ok(ProtostarCasm { prime: prime, bytecode: bytecode, hints: hints, test_entry_points: test_entry_points })
    }

    // Copied from crates/cairo-lang-runner/src/lib.rs
    /// Finds first function ending with `name_suffix`.
    fn find_function(&self, name_suffix: &str) -> Result<&Function, GeneratorError> {
        self.sierra_program
            .funcs
            .iter()
            .find(|f| {
                if let Some(name) = &f.id.debug_name { name.ends_with(name_suffix) } else { false }
            })
            .ok_or_else(|| GeneratorError::MissingFunction { suffix: name_suffix.to_owned() })
    }

    // Copied from crates/cairo-lang-runner/src/lib.rs
    /// Returns the instructions to add to the beginning of the code to successfully call the main
    /// function, as well as the builtins required to execute the program.
    fn create_entry_code(
        &self,
        func: &Function,
        args: &[BigInt],
        initial_gas: usize,
    ) -> Result<(Vec<Instruction>, Vec<String>), GeneratorError> {
        let mut arg_iter = args.iter();
        let mut expected_arguments_size = 0;
        let mut ctx = casm! {};
        // The builtins in the formatting expected by the runner.
        let builtins: Vec<_> = ["pedersen", "range_check", "bitwise", "ec_op"]
            .map(&str::to_string)
            .into_iter()
            .collect();
        // The offset [fp - i] for each of this builtins in this configuration.
        let builtin_offset: HashMap<cairo_lang_sierra::ids::ConcreteTypeId, i16> = HashMap::from([
            (cairo_lang_sierra::ids::ConcreteTypeId::new_inline("Pedersen"), 6),
            (cairo_lang_sierra::ids::ConcreteTypeId::new_inline("RangeCheck"), 5),
            (cairo_lang_sierra::ids::ConcreteTypeId::new_inline("Bitwise"), 4),
            (cairo_lang_sierra::ids::ConcreteTypeId::new_inline("EcOp"), 3),
        ]);
        if func.signature.param_types.contains(&"DictManager".into()) {
            casm_extend! {ctx,
                // DictManager segment.
                %{ memory[ap + 0] = segments.add() %}
                // DictInfos segment.
                %{ memory[ap + 1] = segments.add() %}
                ap += 2;
                [ap + 0] = 0, ap++;
                // Write DictInfos segment, n_dicts (0), and n_destructed (0) to the DictManager segment.
                [ap - 2] = [[ap - 3]];
                [ap - 1] = [[ap - 3] + 1];
                [ap - 1] = [[ap - 3] + 2];
            }
        }
        for (i, ty) in func.signature.param_types.iter().enumerate() {
            if let Some(offset) = builtin_offset.get(ty) {
                casm_extend! {ctx,
                    [ap + 0] = [fp - offset], ap++;
                }
            } else if ty == &"System".into() {
                casm_extend! {ctx,
                    %{ memory[ap + 0] = segments.add() %}
                    ap += 1;
                }
            } else if ty == &"GasBuiltin".into() {
                casm_extend! {ctx,
                    [ap + 0] = initial_gas, ap++;
                }
            } else if ty == &"DictManager".into() {
                let offset = -(i as i16) - 3;
                casm_extend! {ctx,
                    [ap + 0] = [ap + offset], ap++;
                }
            } else {
                let arg_size = self.sierra_program_registry.get_type(ty)?.info().size;
                expected_arguments_size += arg_size as usize;
                for _ in 0..arg_size {
                    if let Some(value) = arg_iter.next() {
                        casm_extend! {ctx,
                            [ap + 0] = (value.clone()), ap++;
                        }
                    }
                }
            }
        }
        if expected_arguments_size != args.len() {
            return Err(GeneratorError::ArgumentsSizeMismatch {
                expected: expected_arguments_size,
                actual: args.len(),
            });
        }
        let before_final_call = ctx.current_code_offset;
        let final_call_size = 3;
        let offset = final_call_size
            + self.casm_program.debug_info.sierra_statement_info[func.entry_point.0].code_offset;
        casm_extend! {ctx,
            call rel offset;
            ret;
        }
        assert_eq!(before_final_call + final_call_size, ctx.current_code_offset);
        Ok((ctx.instructions, builtins))
    }

    // Copied from crates/cairo-lang-runner/src/lib.rs
    /// Creates a list of instructions that will be appended to the program's bytecode.
    pub fn create_code_footer(&self) -> Vec<Instruction> {
        casm! {
            // Add a `ret` instruction used in libfuncs that retrieve the current value of the `fp`
            // and `pc` registers.
            ret;
        }
        .instructions
    }
}
fn create_metadata(
    sierra_program: &cairo_lang_sierra::program::Program,
    calc_gas: bool,
) -> Result<Metadata, GeneratorError> {
    let gas_info = if calc_gas {
        calc_gas_info(sierra_program).map_err(|_| GeneratorError::FailedGasCalculation)?
    } else {
        GasInfo { variable_values: HashMap::new(), function_costs: HashMap::new() }
    };
    let metadata = Metadata { ap_change_info: calc_ap_changes(sierra_program)?, gas_info };
    Ok(metadata)
}