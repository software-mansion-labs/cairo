#[cfg(test)]
#[path = "reorder_statements_test.rs"]
mod test;

use std::cmp::Reverse;

use cairo_lang_semantic::corelib;
use cairo_lang_utils::ordered_hash_map::OrderedHashMap;
use cairo_lang_utils::unordered_hash_set::UnorderedHashSet;
use itertools::Itertools;

use crate::borrow_check::analysis::{Analyzer, BackAnalysis, StatementLocation};
use crate::db::LoweringGroup;
use crate::ids::{FunctionId, FunctionLongId};
use crate::{
    BlockId, FlatLowered, MatchInfo, Statement, StatementCall, VarRemapping, VarUsage, VariableId,
};

/// Reorder the statments in the lowering in order to move variable definitions closer to their
/// usage. Statement with no side effects and unused outputs are removed.
///
/// The list of call statements that can be moved is currently hardcoded.
///
/// Removing unnessary remapping before this optimization will result in better code.
pub fn reorder_statements(db: &dyn LoweringGroup, lowered: &mut FlatLowered) {
    if !lowered.blocks.is_empty() {
        let semantic_db = db.upcast();
        let bool_not_func_id = db.intern_lowering_function(FunctionLongId::Semantic(
            corelib::get_core_function_id(semantic_db, "bool_not_impl".into(), vec![]),
        ));

        let ctx = ReorderStatementsContext {
            lowered: &*lowered,
            moveable_functions: [bool_not_func_id].into_iter().collect(),
            statement_to_move: vec![],
        };
        let mut analysis =
            BackAnalysis { lowered: &*lowered, block_info: Default::default(), analyzer: ctx };
        analysis.get_root_info();
        let ctx = analysis.analyzer;

        let mut changes_by_block =
            OrderedHashMap::<BlockId, Vec<(usize, Option<Statement>)>>::default();

        for (src, opt_dst) in ctx.statement_to_move.into_iter() {
            changes_by_block.entry(src.0).or_insert_with(Vec::new).push((src.1, None));

            if let Some(dst) = opt_dst {
                let statement = lowered.blocks[src.0].statements[src.1].clone();
                changes_by_block
                    .entry(dst.0)
                    .or_insert_with(Vec::new)
                    .push((dst.1, Some(statement)));
            }
        }

        for (block_id, block_changes) in changes_by_block.into_iter() {
            let statments = &mut lowered.blocks[block_id].statements;

            // Apply block changes in revese order to prevent a change from invalidating the
            // indices of the other changes.
            for (index, opt_statment) in
                block_changes.into_iter().sorted_by_key(|(index, _)| Reverse(*index))
            {
                match opt_statment {
                    Some(stmt) => statments.insert(index, stmt),
                    None => {
                        statments.remove(index);
                    }
                }
            }
        }
    }
}

#[derive(Clone, Default)]
pub struct ReorderStatementsInfo {
    // A mapping from var_id to a candidate location that it can be moved to.
    // If the variable is used in multiple match arms we define the next use to be
    // the match.
    next_use: OrderedHashMap<VariableId, StatementLocation>,
}

pub struct ReorderStatementsContext<'a> {
    lowered: &'a FlatLowered,
    // A list of function that can be moved.
    moveable_functions: UnorderedHashSet<FunctionId>,
    statement_to_move: Vec<(StatementLocation, Option<StatementLocation>)>,
}
impl ReorderStatementsContext<'_> {
    fn call_can_be_moved(&mut self, stmt: &StatementCall) -> bool {
        self.moveable_functions.contains(&stmt.function)
    }
}
impl Analyzer<'_> for ReorderStatementsContext<'_> {
    type Info = ReorderStatementsInfo;

    fn visit_stmt(
        &mut self,
        info: &mut Self::Info,
        statement_location: StatementLocation,
        stmt: &Statement,
    ) {
        let var_to_move = match stmt {
            Statement::Call(stmt) if self.call_can_be_moved(stmt) => {
                assert_eq!(stmt.outputs.len(), 1, "Only calls with a single output can be moved");
                stmt.outputs[0]
            }
            Statement::Literal(stmt) => stmt.output,
            Statement::StructConstruct(stmt) if stmt.inputs.is_empty() => stmt.output,
            Statement::StructDestructure(stmt)
                if self.lowered.variables[stmt.input.var_id].droppable.is_ok()
                    && stmt.outputs.iter().all(|var_id| !info.next_use.contains_key(var_id)) =>
            {
                self.statement_to_move.push((statement_location, None));
                return;
            }
            _ => {
                for var_usage in stmt.inputs() {
                    info.next_use.insert(var_usage.var_id, statement_location);
                }
                return;
            }
        };

        let optional_target_location = info.next_use.swap_remove(&var_to_move);
        if let Some(target_location) = optional_target_location {
            // If the statement is not removed add demand for its inputs.
            for var_usage in stmt.inputs() {
                match info.next_use.entry(var_usage.var_id) {
                    indexmap::map::Entry::Occupied(mut e) => {
                        // Since we don't know where `e.get()` and `target_locaton` converge
                        // we use `statement_location` as a conservative estimate.
                        &e.insert(statement_location)
                    }
                    indexmap::map::Entry::Vacant(e) => e.insert(target_location),
                };
            }
        }

        self.statement_to_move.push((statement_location, optional_target_location));
    }

    fn visit_goto(
        &mut self,
        info: &mut Self::Info,
        statement_location: StatementLocation,
        _target_block_id: BlockId,
        remapping: &VarRemapping,
    ) {
        for VarUsage { var_id, .. } in remapping.values() {
            info.next_use.insert(*var_id, statement_location);
        }
    }

    fn merge_match(
        &mut self,
        statement_location: StatementLocation,
        match_info: &MatchInfo,
        infos: &[Self::Info],
    ) -> Self::Info {
        let mut info = Self::Info::default();

        for arm_info in infos {
            for (var_id, location) in arm_info.next_use.iter() {
                match info.next_use.entry(*var_id) {
                    indexmap::map::Entry::Occupied(mut e) => {
                        // A variable that is used in multiple arms can be moved to
                        // before the match.
                        e.insert(statement_location);
                    }
                    indexmap::map::Entry::Vacant(e) => {
                        e.insert(*location);
                    }
                }
            }
        }

        for var_usage in match_info.inputs() {
            info.next_use.insert(var_usage.var_id, statement_location);
        }

        info
    }

    fn info_from_return(
        &mut self,
        statement_location: StatementLocation,
        vars: &[VarUsage],
    ) -> Self::Info {
        let mut info = Self::Info::default();
        for var_usage in vars {
            info.next_use.insert(var_usage.var_id, statement_location);
        }
        info
    }

    fn info_from_panic(
        &mut self,
        _statement_location: StatementLocation,
        _data: &VarUsage,
    ) -> Self::Info {
        unreachable!("Panics should have been stripped in a previous phase.");
    }
}
