use alloc::string::ToString;

use vm_core::{Felt, Operation::*, WORD_SIZE};

use super::{push_felt, push_u32_value, validate_param, BasicBlockBuilder};
use crate::{assembler::ProcedureContext, diagnostics::Report, AssemblyError};

// INSTRUCTION PARSERS
// ================================================================================================

/// Appends operations to the span needed to execute a memory read instruction. This includes
/// reading a single element or an entire word from either local or global memory. Specifically,
/// this handles mem_load, mem_loadw, loc_load, and loc_loadw instructions.
///
/// VM cycles per operation:
/// - mem_load(w): 1 cycle
/// - mem_load(w).b: 2 cycles
/// - loc_load(w).b:
///    - 4 cycles if b = 1
///    - 3 cycles if b != 1
///
/// # Errors
/// Returns an error if we are reading from local memory and local memory index is greater than
/// the number of procedure locals.
pub fn mem_read(
    block_builder: &mut BasicBlockBuilder,
    proc_ctx: &ProcedureContext,
    addr: Option<u32>,
    is_local: bool,
    is_single: bool,
) -> Result<(), AssemblyError> {
    // if the address was provided as an immediate value, put it onto the stack
    if let Some(addr) = addr {
        if is_local {
            let num_locals = proc_ctx.num_locals();
            local_to_absolute_addr(block_builder, addr as u16, num_locals)?;
        } else {
            push_u32_value(block_builder, addr);
        }
    } else {
        assert!(!is_local, "local always contains addr value");
    }

    // load from the memory address on top of the stack
    if is_single {
        block_builder.push_op(MLoad);
    } else {
        block_builder.push_op(MLoadW);
    }

    Ok(())
}

/// Appends operations to the span needed to execute a memory write instruction with an immediate
/// address. This includes writing a single element or an entire word into either local or global
/// memory. Specifically, this handles mem_store, mem_storew, loc_store, and loc_storew
/// instructions.
///
/// VM cycles per operation:
/// - mem_store.b:
///   - 4 cycles if b = 1
///   - 3 cycles if b != 1
/// - mem_storew.b:
///   - 3 cycles if b = 1
///   - 2 cycles if b != 1
/// - loc_store.b:
///   - 5 cycles if b = 1
///   - 4 cycles if b != 1
/// - loc_storew.b:
///   - 4 cycles if b = 1
///   - 3 cycles if b != 1
///
/// # Errors
/// Returns an error if we are writing to local memory and local memory index is greater than
/// the number of procedure locals.
pub fn mem_write_imm(
    block_builder: &mut BasicBlockBuilder,
    proc_ctx: &ProcedureContext,
    addr: u32,
    is_local: bool,
    is_single: bool,
) -> Result<(), AssemblyError> {
    if is_local {
        local_to_absolute_addr(block_builder, addr as u16, proc_ctx.num_locals())?;
    } else {
        push_u32_value(block_builder, addr);
    }

    if is_single {
        block_builder.push_op(MStore);
        block_builder.push_op(Drop);
    } else {
        block_builder.push_op(MStoreW);
    }

    Ok(())
}

// HELPER FUNCTIONS
// ================================================================================================

/// Appends a sequence of operations to the span needed for converting procedure local index to
/// absolute memory address. This consists of putting index onto the stack and then executing
/// LOCADDR operation.
///
/// This operation takes:
/// - 3 VM cycles if index == 1
/// - 2 VM cycles if index != 1
///
/// # Errors
/// Returns an error if index is greater than the number of procedure locals.
pub fn local_to_absolute_addr(
    block_builder: &mut BasicBlockBuilder,
    index_of_local: u16,
    num_proc_locals: u16,
) -> Result<(), AssemblyError> {
    if num_proc_locals == 0 {
        return Err(AssemblyError::Other(
            Report::msg(
                "number of procedure locals was not set (or set to 0), but local values were used"
                    .to_string(),
            )
            .into(),
        ));
    }

    let max = num_proc_locals - 1;
    validate_param(index_of_local, 0..=max)?;

    let fmp_offset_of_local = (max - index_of_local) * WORD_SIZE as u16;
    push_felt(block_builder, -Felt::from(fmp_offset_of_local));
    block_builder.push_op(FmpAdd);

    Ok(())
}
