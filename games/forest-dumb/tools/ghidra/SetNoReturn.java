// Test only (translator T43): mark functions non-returning, as Ghidra's no-return analysis does on the
// game to load_file (data after enough of its calls), so the export's handling of the calls Ghidra
// then leaves without a fall-through is tested on a made-up program, where analysis marks none.
// Args: <hNN+$off>...   e.g. h1+$20
//@category Amiga

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.mem.MemoryBlock;

public class SetNoReturn extends GhidraScript {
    /** The Amiga loader's block of hunk `n`: CODE_00, DATA_01, ... */
    private MemoryBlock hunk(int n) {
        for (MemoryBlock b : currentProgram.getMemory().getBlocks())
            if (b.getName().matches("^[A-Z]+_0*" + n + "$")) return b;
        throw new IllegalArgumentException("no block for hunk " + n);
    }

    @Override
    public void run() throws Exception {
        for (String arg : getScriptArgs()) {
            String[] f = arg.substring(1).split("\\+\\$");
            Address a = hunk(Integer.parseInt(f[0])).getStart().add(Long.parseLong(f[1], 16));
            Function fn = getFunctionAt(a);
            if (fn == null) {
                disassemble(a);
                fn = createFunction(a, null);
            }
            if (fn == null) throw new IllegalStateException(arg + ": no function");
            fn.setNoReturn(true);
            println("no-return " + fn.getName() + " at " + arg);
        }
    }
}
