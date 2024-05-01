Given ω, accumulator ν and evaluation point τ on the top of the stack, this routine first computes<br />⍳ = ω / (τ - ω). It then loads four elements from memory address (q_ptr + 1) i.e. a0, a1, a2, a3<br />and finally accumulates it into ν, while consuming <a1, a0> only when j is even | j ∈ [0..64)<br /><br />Where:<br /><a0, a1, a2, a3> loaded from (q_ptr + 1) s.t. a1, a0 are consumed into ν, and<br />remaining part of word i.e. a3, a2 will be consumed during immediate next iteration<br />of computing β which involves invocation of accumulate_for_odd_index.<br /><br />Input: [ω, ν1, ν0, τ1, τ0, q_ptr - 1, ...]<br />Output: [a3, a2, ν1', ν0', τ1, τ0, q_ptr, ...]<br /><br />Cycles: 54<br />
## std::crypto::fri::ext2fri
| Procedure | Description |
| ----------- | ------------- |
| verify_remainder_64 | Given memory address of the remainder codeword with 64 evaluations, this routine checks<br />probabilistically that this codeword is the evaluation of a degree 7 polynomial.<br /><br />A few assumptions about q_ptr:<br />- q_ptr is an absolute memory address of the beginning of remainder codeword.<br />- Each evaluation is 2 elements wide because they belong to quadratic extension field (meaning<br />each memory address will hold two consecutive evaluations)<br />- Words (four field elements), in memory, are laid out in this order (a0_0, a0_1, a1_0, a1_1).<br />This means that (a0_1, a0_0) -> first evaluation and (a1_1, a1_0) -> next evaluation<br />- Next 31 memory addresses should be holding remaining 62 evaluations. That is, if q_ptr holds<br />(a0_0, a0_1, a1_0, a1_1), then q_ptr + 1, must hold (a2_0, a2_1, a3_0, a3_1), and q_ptr + 31<br />should be holding (a62_0, a62_1, a63_0, a63_1).<br />- The polynomial is laid out starting from memory address q_ptr + 32 and occupies 4 contiguous<br />memory addresses.<br />If remainder verification fails, execution of the program stops.<br /><br />Input: [τ1, τ0, q_ptr, ...]<br />Output: [...]<br /><br />Cycles: 2931<br /> |
| verify_remainder_32 | Given memory address of the remainder codeword with 32 evaluations, this routine checks<br />probabilistically that the codeword is the evaluation of a degree 3 polynomial.<br /><br />A few assumptions about q_ptr:<br />- q_ptr is an absolute memory address of the beginning of remainder codeword.<br />- Each evaluation is 2 elements wide because they belong to quadratic extension field (meaning<br />each memory address will hold two consecutive evaluations)<br />- Words (four field elements), in memory, are laid out in this order (a0_0, a0_1, a1_0, a1_1).<br />This means that (a0_1, a0_0) -> first evaluation and (a1_1, a1_0) -> next evaluation<br />- Next 15 memory addresses should be holding remaining 30 evaluations. That is, if q_ptr holds<br />(a0_0, a0_1, a1_0, a1_1), then q_ptr + 1, must hold (a2_0, a2_1, a3_0, a3_1), and q_ptr + 15<br />should be holding (a30_0, a30_1, a31_0, a31_1).<br />- The polynomial is laid out starting from memory address q_ptr + 16 and occupies 4 contiguous<br />memory addresses.<br /><br />If remainder verification fails, execution of the program stops.<br /><br />Input: [τ1, τ0, q_ptr, ...]<br />Output: [...]<br /><br />Cycles: 1483<br /> |