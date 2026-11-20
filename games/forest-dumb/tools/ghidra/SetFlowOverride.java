// Test only (translator T07): set a flow override on instructions, as Ghidra's own function analysis
// does on the game when a bra.w lands on another function's start (CALL_RETURN), so the export's
// handling of that override is tested on a made-up program, where analysis sets none.
// Args: <override> <hNN+$off>...   e.g. CALL_RETURN h0+$4a
//@category Amiga

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.FlowOverride;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.mem.MemoryBlock;

public class SetFlowOverride extends GhidraScript {
    /** The Amiga loader's block of hunk `n`: CODE_00, DATA_01, ... */
    private MemoryBlock hunk(int n) {
        for (MemoryBlock b : currentProgram.getMemory().getBlocks())
            if (b.getName().matches("^[A-Z]+_0*" + n + "$")) return b;
        throw new IllegalArgumentException("no block for hunk " + n);
    }

    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        FlowOverride how = FlowOverride.valueOf(args[0]);
        for (int i = 1; i < args.length; i++) {
            String[] f = args[i].substring(1).split("\\+\\$");
            Address a = hunk(Integer.parseInt(f[0])).getStart().add(Long.parseLong(f[1], 16));
            Instruction in = getInstructionAt(a);
            if (in == null) {
                disassemble(a);
                in = getInstructionAt(a);
            }
            if (in == null) throw new IllegalStateException(args[i] + ": no instruction");
            in.setFlowOverride(how);
            println("override " + how + " at " + args[i] + ": " + in + " flow " + in.getFlowType());
        }
    }
}
