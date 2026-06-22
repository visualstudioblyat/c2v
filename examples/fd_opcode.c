/* fd_opcode — MIPS-style opcode decoder using switch/case.
   Tests: switch → mux tree conversion.
   CHStone benchmark: mips (simplified). */

typedef unsigned int uint;

uint
fd_opcode_exec( uint opcode, uint rs, uint rt, uint imm ) {
  uint result;
  switch( opcode ) {
    case 0:  result = rs + rt;           break;  /* ADD */
    case 1:  result = rs - rt;           break;  /* SUB */
    case 2:  result = rs & rt;           break;  /* AND */
    case 3:  result = rs | rt;           break;  /* OR */
    case 4:  result = rs ^ rt;           break;  /* XOR */
    case 5:  result = rs << (rt & 31);   break;  /* SLL */
    case 6:  result = rs >> (rt & 31);   break;  /* SRL */
    case 7:  result = (rs < rt) ? 1 : 0; break;  /* SLT */
    case 8:  result = rs + imm;          break;  /* ADDI */
    case 9:  result = rs & imm;          break;  /* ANDI */
    case 10: result = rs | imm;          break;  /* ORI */
    case 11: result = imm << 16;         break;  /* LUI */
    default: result = 0;                 break;
  }
  return result;
}
