use alloc::vec::Vec;

use miden_air::{
    trace::chiplets::memory::{
        ELEMENT_OR_WORD_COL_IDX, FLAG_SAME_BATCH_AND_CONTEXT, IDX0_COL_IDX, IDX1_COL_IDX,
        MEMORY_ACCESS_ELEMENT, MEMORY_ACCESS_WORD, MEMORY_READ, MEMORY_WRITE, READ_WRITE_COL_IDX,
        TRACE_WIDTH as MEMORY_TRACE_WIDTH,
    },
    RowIndex,
};
use vm_core::{assert_matches, Word, WORD_SIZE};

use super::{
    super::ZERO,
    segment::{MemoryAccessType, MemoryOperation},
    Felt, FieldElement, Memory, TraceFragment, BATCH_COL_IDX, CLK_COL_IDX, CTX_COL_IDX, D0_COL_IDX,
    D1_COL_IDX, D_INV_COL_IDX, EMPTY_WORD, ONE, V_COL_RANGE,
};
use crate::{ContextId, ExecutionError};

#[test]
fn mem_init() {
    let mem = Memory::default();
    assert_eq!(0, mem.num_accessed_batches());
    assert_eq!(0, mem.trace_len());
}

#[test]
fn mem_read() {
    let mut mem = Memory::default();

    // read a value from address 0; clk = 1
    let addr0 = ZERO;
    let value = mem.read(ContextId::root(), addr0, 1.into()).unwrap();
    assert_eq!(ZERO, value);
    assert_eq!(1, mem.num_accessed_batches());
    assert_eq!(1, mem.trace_len());

    // read a value from address 3; clk = 2
    let addr3 = Felt::from(3_u32);
    let value = mem.read(ContextId::root(), addr3, 2.into()).unwrap();
    assert_eq!(ZERO, value);
    assert_eq!(1, mem.num_accessed_batches());
    assert_eq!(2, mem.trace_len());

    // read a value from address 0 again; clk = 3
    let value = mem.read(ContextId::root(), addr0, 3.into()).unwrap();
    assert_eq!(ZERO, value);
    assert_eq!(1, mem.num_accessed_batches());
    assert_eq!(3, mem.trace_len());

    // read a value from address 2; clk = 4
    let addr2 = Felt::from(2_u32);
    let value = mem.read(ContextId::root(), addr2, 4.into()).unwrap();
    assert_eq!(ZERO, value);
    assert_eq!(1, mem.num_accessed_batches());
    assert_eq!(4, mem.trace_len());

    // check generated trace and memory data provided to the ChipletsBus; rows should be sorted only
    // by clock cycle, since they all access the same batch
    let trace = build_trace(mem, 4);

    // clk 1
    let mut prev_row = [ZERO; MEMORY_TRACE_WIDTH];
    let memory_access = MemoryAccess::new(
        MemoryOperation::Read,
        MemoryAccessType::Element { addr_idx_in_batch: 0 },
        ContextId::root(),
        addr0,
        1.into(),
        EMPTY_WORD,
    );
    prev_row = verify_memory_access(&trace, 0, memory_access, prev_row);

    // clk 2
    let memory_access = MemoryAccess::new(
        MemoryOperation::Read,
        MemoryAccessType::Element { addr_idx_in_batch: 3 },
        ContextId::root(),
        addr3,
        2.into(),
        EMPTY_WORD,
    );
    prev_row = verify_memory_access(&trace, 1, memory_access, prev_row);

    // clk 3
    let memory_access = MemoryAccess::new(
        MemoryOperation::Read,
        MemoryAccessType::Element { addr_idx_in_batch: 0 },
        ContextId::root(),
        addr0,
        3.into(),
        EMPTY_WORD,
    );
    prev_row = verify_memory_access(&trace, 2, memory_access, prev_row);

    // clk 4
    let memory_access = MemoryAccess::new(
        MemoryOperation::Read,
        MemoryAccessType::Element { addr_idx_in_batch: 2 },
        ContextId::root(),
        addr2,
        4.into(),
        EMPTY_WORD,
    );
    verify_memory_access(&trace, 3, memory_access, prev_row);
}

/// Tests that writing a word to an address that is not aligned with the word boundary results in an
/// error.
#[test]
fn mem_read_word_unaligned() {
    let mut mem = Memory::default();

    // write a value into address 0; clk = 1
    let addr = ONE;
    let clk = 1.into();
    let ctx = ContextId::root();
    let ret = mem.read_word(ctx, addr, clk);

    assert_matches!(
        ret,
        Err(ExecutionError::MemoryUnalignedWordAccess { addr: _, ctx: _, clk: _ })
    );
}

#[test]
fn mem_write() {
    let mut mem = Memory::default();

    // write a value into address 0; clk = 1
    let addr0 = 0_u32;
    let word1 = [ONE, ZERO, ZERO, ZERO];
    mem.write_word(ContextId::root(), addr0.into(), 1.into(), word1).unwrap();
    assert_eq!(word1, mem.get_word(ContextId::root(), addr0).unwrap().unwrap());
    assert_eq!(1, mem.num_accessed_batches());
    assert_eq!(1, mem.trace_len());

    // write a value into address 2; clk = 2
    let addr2 = 2_u32;
    let value5 = Felt::new(5);
    mem.write(ContextId::root(), addr2.into(), 2.into(), value5).unwrap();
    assert_eq!(value5, mem.get_value(ContextId::root(), addr2).unwrap());
    assert_eq!(1, mem.num_accessed_batches());
    assert_eq!(2, mem.trace_len());

    // write a value into address 1; clk = 3
    let addr1 = 1_u32;
    let value7 = Felt::new(7);
    mem.write(ContextId::root(), addr1.into(), 3.into(), value7).unwrap();
    assert_eq!(value7, mem.get_value(ContextId::root(), addr1).unwrap());
    assert_eq!(1, mem.num_accessed_batches());
    assert_eq!(3, mem.trace_len());

    // write a value into address 3; clk = 4
    let addr3 = 3_u32;
    let value9 = Felt::new(9);
    mem.write(ContextId::root(), addr3.into(), 4.into(), value9).unwrap();
    assert_eq!(value9, mem.get_value(ContextId::root(), addr3).unwrap());
    assert_eq!(1, mem.num_accessed_batches());
    assert_eq!(4, mem.trace_len());

    // write a word into address 4; clk = 5
    let addr4 = 4_u32;
    let word1234 = [ONE, 2_u32.into(), 3_u32.into(), 4_u32.into()];
    mem.write_word(ContextId::root(), addr4.into(), 5.into(), word1234).unwrap();
    assert_eq!(word1234, mem.get_word(ContextId::root(), addr4).unwrap().unwrap());
    assert_eq!(2, mem.num_accessed_batches());
    assert_eq!(5, mem.trace_len());

    // write a word into address 0; clk = 6
    let word5678: [Felt; 4] = [5_u32.into(), 6_u32.into(), 7_u32.into(), 8_u32.into()];
    mem.write_word(ContextId::root(), addr0.into(), 6.into(), word5678).unwrap();
    assert_eq!(word5678, mem.get_word(ContextId::root(), addr0).unwrap().unwrap());
    assert_eq!(2, mem.num_accessed_batches());
    assert_eq!(6, mem.trace_len());

    // check generated trace and memory data provided to the ChipletsBus; rows should be sorted by
    // address and then clock cycle
    let trace = build_trace(mem, 6);

    // batch 0
    let mut prev_row = [ZERO; MEMORY_TRACE_WIDTH];
    let memory_access = MemoryAccess::new(
        MemoryOperation::Write,
        MemoryAccessType::Word,
        ContextId::root(),
        addr0.into(),
        1.into(),
        word1,
    );
    prev_row = verify_memory_access(&trace, 0, memory_access, prev_row);

    let memory_access = MemoryAccess::new(
        MemoryOperation::Write,
        MemoryAccessType::Element { addr_idx_in_batch: 2 },
        ContextId::root(),
        addr2.into(),
        2.into(),
        [ONE, ZERO, value5, ZERO],
    );
    prev_row = verify_memory_access(&trace, 1, memory_access, prev_row);

    let memory_access = MemoryAccess::new(
        MemoryOperation::Write,
        MemoryAccessType::Element { addr_idx_in_batch: 1 },
        ContextId::root(),
        addr1.into(),
        3.into(),
        [ONE, value7, value5, ZERO],
    );
    prev_row = verify_memory_access(&trace, 2, memory_access, prev_row);

    let memory_access = MemoryAccess::new(
        MemoryOperation::Write,
        MemoryAccessType::Element { addr_idx_in_batch: 3 },
        ContextId::root(),
        addr3.into(),
        4.into(),
        [ONE, value7, value5, value9],
    );
    prev_row = verify_memory_access(&trace, 3, memory_access, prev_row);

    let memory_access = MemoryAccess::new(
        MemoryOperation::Write,
        MemoryAccessType::Word,
        ContextId::root(),
        addr0.into(),
        6.into(),
        word5678,
    );
    prev_row = verify_memory_access(&trace, 4, memory_access, prev_row);

    // batch 1
    let memory_access = MemoryAccess::new(
        MemoryOperation::Write,
        MemoryAccessType::Word,
        ContextId::root(),
        addr4.into(),
        5.into(),
        word1234,
    );
    verify_memory_access(&trace, 5, memory_access, prev_row);
}

/// Tests that writing a word to an address that is not aligned with the word boundary results in an
/// error.
#[test]
fn mem_write_word_unaligned() {
    let mut mem = Memory::default();

    // write a value into address 0; clk = 1
    let addr = ONE;
    let word1 = [ONE, ZERO, ZERO, ZERO];
    let clk = 1.into();
    let ctx = ContextId::root();
    let ret = mem.write_word(ctx, addr, clk, word1);

    assert_matches!(
        ret,
        Err(ExecutionError::MemoryUnalignedWordAccess { addr: _, ctx: _, clk: _ })
    );
}

/// Tests that values written are properly read back.
#[test]
fn mem_write_read() {
    let mut mem = Memory::default();
    let mut clk: RowIndex = 1.into();

    // write [1,2,3,4] starting at address 0; clk = 1
    let word1234 = [ONE, 2_u32.into(), 3_u32.into(), 4_u32.into()];
    mem.write_word(ContextId::root(), ZERO, clk, word1234).unwrap();
    clk += 1;

    // read individual values from addresses 3,2,1,0; clk = 2,3,4,5
    let value_read = mem.read(ContextId::root(), 3_u32.into(), clk).unwrap();
    assert_eq!(value_read, 4_u32.into());
    clk += 1;
    let value_read = mem.read(ContextId::root(), 2_u32.into(), clk).unwrap();
    assert_eq!(value_read, 3_u32.into());
    clk += 1;
    let value_read = mem.read(ContextId::root(), 1_u32.into(), clk).unwrap();
    assert_eq!(value_read, 2_u32.into());
    clk += 1;
    let value_read = mem.read(ContextId::root(), ZERO, clk).unwrap();
    assert_eq!(value_read, 1_u32.into());
    clk += 1;

    // read word from address 0; clk = 6
    let word_read = mem.read_word(ContextId::root(), ZERO, clk).unwrap();
    assert_eq!(word_read, word1234);
    clk += 1;

    // write 42 into address 2; clk = 7
    mem.write(ContextId::root(), 2_u32.into(), clk, 42_u32.into()).unwrap();
    clk += 1;

    // read element from address 2; clk = 8
    let value_read = mem.read(ContextId::root(), 2_u32.into(), clk).unwrap();
    assert_eq!(value_read, 42_u32.into());
    clk += 1;

    // read word from address 0; clk = 9
    let word_read = mem.read_word(ContextId::root(), ZERO, clk).unwrap();
    assert_eq!(word_read, [ONE, 2_u32.into(), 42_u32.into(), 4_u32.into()]);
    clk += 1;

    // check generated trace and memory data provided to the ChipletsBus; rows should be sorted by
    // address and then clock cycle
    let trace = build_trace(mem, 9);
    let mut clk: RowIndex = 1.into();

    // address 2
    let mut prev_row = [ZERO; MEMORY_TRACE_WIDTH];
    let memory_access = MemoryAccess::new(
        MemoryOperation::Write,
        MemoryAccessType::Word,
        ContextId::root(),
        ZERO,
        clk,
        word1234,
    );
    prev_row = verify_memory_access(&trace, 0, memory_access, prev_row);
    clk += 1;

    let memory_access = MemoryAccess::new(
        MemoryOperation::Read,
        MemoryAccessType::Element { addr_idx_in_batch: 3 },
        ContextId::root(),
        3_u32.into(),
        clk,
        word1234,
    );
    prev_row = verify_memory_access(&trace, 1, memory_access, prev_row);
    clk += 1;

    let memory_access = MemoryAccess::new(
        MemoryOperation::Read,
        MemoryAccessType::Element { addr_idx_in_batch: 2 },
        ContextId::root(),
        2_u32.into(),
        clk,
        word1234,
    );
    prev_row = verify_memory_access(&trace, 2, memory_access, prev_row);
    clk += 1;

    let memory_access = MemoryAccess::new(
        MemoryOperation::Read,
        MemoryAccessType::Element { addr_idx_in_batch: 1 },
        ContextId::root(),
        1_u32.into(),
        clk,
        word1234,
    );
    prev_row = verify_memory_access(&trace, 3, memory_access, prev_row);
    clk += 1;

    let memory_access = MemoryAccess::new(
        MemoryOperation::Read,
        MemoryAccessType::Element { addr_idx_in_batch: 0 },
        ContextId::root(),
        ZERO,
        clk,
        word1234,
    );
    prev_row = verify_memory_access(&trace, 4, memory_access, prev_row);
    clk += 1;

    let memory_access = MemoryAccess::new(
        MemoryOperation::Read,
        MemoryAccessType::Word,
        ContextId::root(),
        ZERO,
        clk,
        word1234,
    );
    prev_row = verify_memory_access(&trace, 5, memory_access, prev_row);
    clk += 1;

    let memory_access = MemoryAccess::new(
        MemoryOperation::Write,
        MemoryAccessType::Element { addr_idx_in_batch: 2 },
        ContextId::root(),
        2_u32.into(),
        clk,
        [ONE, 2_u32.into(), 42_u32.into(), 4_u32.into()],
    );
    prev_row = verify_memory_access(&trace, 6, memory_access, prev_row);
    clk += 1;

    let memory_access = MemoryAccess::new(
        MemoryOperation::Read,
        MemoryAccessType::Element { addr_idx_in_batch: 2 },
        ContextId::root(),
        2_u32.into(),
        clk,
        [ONE, 2_u32.into(), 42_u32.into(), 4_u32.into()],
    );
    prev_row = verify_memory_access(&trace, 7, memory_access, prev_row);
    clk += 1;

    let memory_access = MemoryAccess::new(
        MemoryOperation::Read,
        MemoryAccessType::Word,
        ContextId::root(),
        ZERO,
        clk,
        [ONE, 2_u32.into(), 42_u32.into(), 4_u32.into()],
    );
    verify_memory_access(&trace, 8, memory_access, prev_row);
}

#[test]
fn mem_get_state_at() {
    let mut mem = Memory::default();

    let addr_start: u32 = 40_u32;

    // Write word starting at (ctx = 0, addr = 40) at clk = 1.
    // This means that mem[40..43] is set at the beginning of clk = 2
    let word1234 = [ONE, 2_u32.into(), 3_u32.into(), 4_u32.into()];
    mem.write_word(ContextId::root(), addr_start.into(), 1.into(), word1234)
        .unwrap();

    let word4567: [Felt; 4] = [4_u32.into(), 5_u32.into(), 6_u32.into(), 7_u32.into()];
    mem.write_word(ContextId::root(), addr_start.into(), 2.into(), word4567)
        .unwrap();

    // Check memory state at clk = 2
    let clk: RowIndex = 2.into();
    assert_eq!(
        mem.get_state_at(ContextId::root(), clk),
        vec![
            (addr_start.into(), word1234[0]),
            (u64::from(addr_start) + 1_u64, word1234[1]),
            (u64::from(addr_start) + 2_u64, word1234[2]),
            (u64::from(addr_start) + 3_u64, word1234[3])
        ]
    );
    assert_eq!(mem.get_state_at(3.into(), clk), vec![]);

    // Check memory state at clk = 3
    let clk: RowIndex = 3.into();
    assert_eq!(
        mem.get_state_at(ContextId::root(), clk),
        vec![
            (addr_start.into(), word4567[0]),
            (u64::from(addr_start) + 1_u64, word4567[1]),
            (u64::from(addr_start) + 2_u64, word4567[2]),
            (u64::from(addr_start) + 3_u64, word4567[3])
        ]
    );
    assert_eq!(mem.get_state_at(3.into(), clk), vec![]);
}

// HELPER STRUCT & FUNCTIONS
// ================================================================================================

/// Contains data representing a memory access.
pub struct MemoryAccess {
    operation: MemoryOperation,
    access_type: MemoryAccessType,
    ctx: ContextId,
    addr: Felt,
    clk: Felt,
    batch_values: [Felt; 4],
}

impl MemoryAccess {
    pub fn new(
        operation: MemoryOperation,
        access_type: MemoryAccessType,
        ctx: ContextId,
        addr: Felt,
        clk: RowIndex,
        batch_values: Word,
    ) -> Self {
        if let MemoryAccessType::Element { addr_idx_in_batch: addr_idx_in_word } = access_type {
            let addr: u32 = addr.try_into().unwrap();
            assert_eq!(addr_idx_in_word as u32, addr % WORD_SIZE as u32);
        }

        Self {
            operation,
            access_type,
            ctx,
            addr,
            clk: Felt::from(clk),
            batch_values,
        }
    }
}

/// Builds a trace of the specified length and fills it with data from the provided Memory instance.
fn build_trace(mem: Memory, num_rows: usize) -> Vec<Vec<Felt>> {
    let mut trace = (0..MEMORY_TRACE_WIDTH).map(|_| vec![ZERO; num_rows]).collect::<Vec<_>>();
    let mut fragment = TraceFragment::trace_to_fragment(&mut trace);
    mem.fill_trace(&mut fragment);

    trace
}

fn read_trace_row(trace: &[Vec<Felt>], step: usize) -> [Felt; MEMORY_TRACE_WIDTH] {
    let mut row = [ZERO; MEMORY_TRACE_WIDTH];
    for (value, column) in row.iter_mut().zip(trace) {
        *value = column[step];
    }
    row
}

fn build_trace_row(
    memory_access: MemoryAccess,
    prev_row: [Felt; MEMORY_TRACE_WIDTH],
) -> [Felt; MEMORY_TRACE_WIDTH] {
    let MemoryAccess {
        operation,
        access_type,
        ctx,
        addr,
        clk,
        batch_values,
    } = memory_access;

    let (batch, idx1, idx0) = {
        let addr: u32 = addr.try_into().unwrap();
        let remainder = addr % WORD_SIZE as u32;
        let batch = Felt::from(addr - remainder);

        match remainder {
            0 => (batch, ZERO, ZERO),
            1 => (batch, ZERO, ONE),
            2 => (batch, ONE, ZERO),
            3 => (batch, ONE, ONE),
            _ => unreachable!(),
        }
    };

    let mut row = [ZERO; MEMORY_TRACE_WIDTH];

    row[READ_WRITE_COL_IDX] = match operation {
        MemoryOperation::Read => MEMORY_READ,
        MemoryOperation::Write => MEMORY_WRITE,
    };
    row[ELEMENT_OR_WORD_COL_IDX] = match access_type {
        MemoryAccessType::Element { .. } => MEMORY_ACCESS_ELEMENT,
        MemoryAccessType::Word => MEMORY_ACCESS_WORD,
    };
    row[CTX_COL_IDX] = ctx.into();
    row[BATCH_COL_IDX] = batch;
    row[IDX0_COL_IDX] = idx0;
    row[IDX1_COL_IDX] = idx1;
    row[CLK_COL_IDX] = clk;
    row[V_COL_RANGE.start] = batch_values[0];
    row[V_COL_RANGE.start + 1] = batch_values[1];
    row[V_COL_RANGE.start + 2] = batch_values[2];
    row[V_COL_RANGE.start + 3] = batch_values[3];

    if prev_row != [ZERO; MEMORY_TRACE_WIDTH] {
        let delta = if row[CTX_COL_IDX] != prev_row[CTX_COL_IDX] {
            row[CTX_COL_IDX] - prev_row[CTX_COL_IDX]
        } else if row[BATCH_COL_IDX] != prev_row[BATCH_COL_IDX] {
            row[BATCH_COL_IDX] - prev_row[BATCH_COL_IDX]
        } else {
            row[CLK_COL_IDX] - prev_row[CLK_COL_IDX] - ONE
        };

        let (hi, lo) = super::split_element_u32_into_u16(delta);
        row[D0_COL_IDX] = lo;
        row[D1_COL_IDX] = hi;
        row[D_INV_COL_IDX] = delta.inv();
    }

    if row[BATCH_COL_IDX] == prev_row[BATCH_COL_IDX] && row[CTX_COL_IDX] == prev_row[CTX_COL_IDX] {
        row[FLAG_SAME_BATCH_AND_CONTEXT] = ONE;
    } else {
        row[FLAG_SAME_BATCH_AND_CONTEXT] = ZERO;
    }

    row
}

fn verify_memory_access(
    trace: &[Vec<Felt>],
    row: u32,
    mem_access: MemoryAccess,
    prev_row: [Felt; MEMORY_TRACE_WIDTH],
) -> [Felt; MEMORY_TRACE_WIDTH] {
    let expected_row = build_trace_row(mem_access, prev_row);
    let actual_row = read_trace_row(trace, row as usize);
    assert_eq!(expected_row, actual_row);

    expected_row
}
