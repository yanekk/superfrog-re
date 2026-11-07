// Post-script: exports Ghidra's P-code for every instruction reached by direct flow from a list of
// entries, with every address written hunk-relative, so code generated from it does not depend on
// where Ghidra (or anything else) loaded the program. Names no game: every fact arrives as an argument.
//
// Args: <entries.tsv> <relocs.tsv> <out.json>
//   entries.tsv: hunk \t offset                              (decimal; one entry a line)
//   relocs.tsv:  hunk \t offset \t target hunk \t value      (decimal; the file's HUNK_RELOC32 entries,
//                value = the long the file holds there, an offset into the target hunk)
//
// The relocations come from the caller because Ghidra's Amiga loader records none in its relocation
// table. A relocated long is found among an instruction's P-code operands by its loaded value.
//@category Amiga
import ghidra.app.script.GhidraScript;
import ghidra.app.util.PseudoDisassembler;
import ghidra.app.util.PseudoInstruction;
import ghidra.program.model.address.Address;
import ghidra.program.model.address.AddressSpace;
import ghidra.program.model.listing.Data;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.mem.MemoryBlock;
import ghidra.program.model.pcode.PcodeOp;
import ghidra.program.model.pcode.Varnode;
import ghidra.program.model.symbol.FlowType;
import ghidra.program.model.symbol.Reference;

import java.io.PrintWriter;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.*;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public class ExportPcode extends GhidraScript {
    private static final Pattern HUNK_BLOCK = Pattern.compile("^[A-Z]+_(\\d+)$");
    private static final int MAX_ROUNDS = 6;

    private List<MemoryBlock> hunks;
    private Memory mem;

    /** Hunk blocks, named CODE_00, DATA_01, BSS_03 ... by the Amiga loader. */
    private List<MemoryBlock> hunkBlocks() {
        List<MemoryBlock> out = new ArrayList<>();
        for (MemoryBlock b : currentProgram.getMemory().getBlocks()) {
            Matcher m = HUNK_BLOCK.matcher(b.getName());
            if (!m.matches()) continue;
            int n = Integer.parseInt(m.group(1));
            while (out.size() <= n) out.add(null);
            out.set(n, b);
        }
        return out;
    }

    private int hunkOf(long addr) {
        for (int i = 0; i < hunks.size(); i++) {
            MemoryBlock b = hunks.get(i);
            if (b == null) continue;
            long s = b.getStart().getOffset();
            if (addr >= s && addr < s + b.getSize()) return i;
        }
        return -1;
    }

    private String rel(long addr) {
        int h = hunkOf(addr);
        return String.format("h%d+$%x", h, addr - hunks.get(h).getStart().getOffset());
    }

    private Address at(int hunk, long off) {
        return hunks.get(hunk).getStart().add(off);
    }

    private static String q(String s) {
        return "\"" + s.replace("\\", "\\\\").replace("\"", "\\\"") + "\"";
    }

    /** What the export needs of one reloc site: its target hunk and the value the file holds. */
    private record Reloc(int hunk, long off, int target, long value) {}

    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        hunks = hunkBlocks();
        mem = currentProgram.getMemory();
        AddressSpace ram = currentProgram.getAddressFactory().getDefaultAddressSpace();

        List<Address> entries = new ArrayList<>();
        for (String line : Files.readAllLines(Path.of(args[0]))) {
            if (line.isBlank()) continue;
            String[] f = line.split("\t");
            entries.add(at(Integer.parseInt(f[0]), Long.parseLong(f[1])));
        }
        // keyed by the Ghidra address of the relocated long
        Map<Long, Reloc> relocs = new HashMap<>();
        for (String line : Files.readAllLines(Path.of(args[1]))) {
            if (line.isBlank()) continue;
            String[] f = line.split("\t");
            Reloc r = new Reloc(Integer.parseInt(f[0]), Long.parseLong(f[1]),
                                Integer.parseInt(f[2]), Long.parseLong(f[3]));
            if (r.hunk < hunks.size() && hunks.get(r.hunk) != null)
                relocs.put(at(r.hunk, r.off).getOffset(), r);
        }

        // Seed the entries, let analysis find what it can (jump tables), then follow direct flow;
        // anything newly disassembled by the walk gets analysed and walked again.
        for (Address a : entries) if (getInstructionAt(a) == null) disassemble(a);
        analyzeChanges(currentProgram);
        TreeMap<Long, Instruction> seen = new TreeMap<>();
        List<String> undecoded = new ArrayList<>();
        for (int round = 0; round < MAX_ROUNDS; round++) {
            boolean grew = false;
            seen.clear();
            undecoded.clear();
            Deque<Address> work = new ArrayDeque<>(entries);
            while (!work.isEmpty()) {
                Address a = work.pop();
                if (seen.containsKey(a.getOffset())) continue;
                if (hunkOf(a.getOffset()) < 0) { undecoded.add(String.format("$%x", a.getOffset())); continue; }
                Instruction in = getInstructionAt(a);
                if (in == null) {
                    disassemble(a);
                    in = getInstructionAt(a);
                    if (in == null) in = clearAndDisassemble(a);
                    grew = true;
                }
                if (in == null) { undecoded.add(rel(a.getOffset())); continue; }
                seen.put(a.getOffset(), in);
                if (in.getDefaultFallThrough() != null) work.push(in.getDefaultFallThrough());
                for (Address t : in.getFlows()) work.push(t);
                for (Address t : computedTargets(in)) work.push(t);
            }
            if (!grew) break;
            analyzeChanges(currentProgram);
        }

        long lo = Long.MAX_VALUE, hi = 0;
        for (MemoryBlock b : hunks) {
            if (b == null) continue;
            lo = Math.min(lo, b.getStart().getOffset());
            hi = Math.max(hi, b.getEnd().getOffset() + 1);
        }

        int unresolved = 0, relocsIn = 0, relocsMatched = 0, pcrel = 0;
        List<String> unmatched = new ArrayList<>(), unexplained = new ArrayList<>(), mismatched = new ArrayList<>();
        Set<Function> functions = new HashSet<>();
        StringBuilder js = new StringBuilder();
        js.append("{\"ghidra_bases\":[");
        for (int i = 0; i < hunks.size(); i++) {
            if (i > 0) js.append(',');
            js.append(hunks.get(i) == null ? "null" : Long.toString(hunks.get(i).getStart().getOffset()));
        }
        js.append("],\n\"entries\":[");
        for (int i = 0; i < entries.size(); i++)
            js.append(i > 0 ? "," : "").append(q(rel(entries.get(i).getOffset())));
        js.append("],\n\"insns\":{");
        boolean firstInsn = true;
        for (Instruction in : seen.values()) {
            long a = in.getAddress().getOffset();
            int len = in.getLength();
            Function fn = getFunctionContaining(in.getAddress());
            if (fn != null) functions.add(fn);
            PcodeOp[] ops = in.getPcode(false);

            // relocated longs inside this instruction, by the value the operands carry
            Map<Long, String> relocValue = new HashMap<>();
            List<long[]> sites = new ArrayList<>();
            for (long s = a; s < a + len; s++) {
                Reloc r = relocs.get(s);
                if (r == null) continue;
                relocsIn++;
                long loaded = mem.getInt(ram.getAddress(s)) & 0xffffffffL;
                long base = hunks.get(r.target).getStart().getOffset();
                // The loader may or may not have added the target's base; either way the operand
                // carries the loaded value, and the file's value is the offset into the target.
                if (loaded != ((base + r.value) & 0xffffffffL) && loaded != r.value)
                    mismatched.add(rel(s));
                String target = String.format("h%d+$%x", r.target, r.value);
                relocValue.put(loaded, target);
                sites.add(new long[]{s, loaded});
            }
            boolean hasPcOperand = in.toString().toUpperCase().contains("PC)");

            StringBuilder o = new StringBuilder("[");
            StringBuilder rl = new StringBuilder("[");
            Set<Long> hit = new HashSet<>();
            for (int i = 0; i < ops.length; i++) {
                PcodeOp op = ops[i];
                boolean flowOp = op.getOpcode() == PcodeOp.BRANCH || op.getOpcode() == PcodeOp.CBRANCH
                        || op.getOpcode() == PcodeOp.CALL;
                if (i > 0) o.append(',');
                o.append("{\"op\":").append(q(op.getMnemonic()));
                o.append(",\"out\":");
                o.append(op.getOutput() == null ? "null"
                        : varnode(op.getOutput(), relocValue, hasPcOperand, false, i, -1, rl, hit, lo, hi, unexplained, a, len));
                o.append(",\"in\":[");
                for (int k = 0; k < op.getNumInputs(); k++) {
                    if (k > 0) o.append(',');
                    o.append(varnode(op.getInput(k), relocValue, hasPcOperand, flowOp && k == 0, i, k, rl, hit,
                                     lo, hi, unexplained, a, len));
                }
                o.append("]}");
            }
            o.append(']');
            rl.append(']');
            for (long[] s : sites) {
                if (hit.contains(s[1])) relocsMatched++;
                else unmatched.add(rel(s[0]));
            }
            pcrel += countPcrel;
            countPcrel = 0;

            // The instruction's own flow, not Ghidra's override: its function analysis turns a bra.w
            // into another function's start into a tail call (CALL_TERMINATOR), which would read as a
            // call that falls through. The P-code (getPcode(false)) is the plain BRANCH either way.
            FlowType ft = rawFlow(in);
            String kind;
            List<String> targets = new ArrayList<>();
            boolean computed = ft.isComputed();
            if (computed) for (Address t : computedTargets(in)) targets.add(rel(t.getOffset()));
            else for (Address t : in.getFlows()) targets.add(hunkOf(t.getOffset()) >= 0 ? rel(t.getOffset())
                                                                 : String.format("$%x", t.getOffset()));
            if (ft.isCall()) kind = "call";
            else if (ft.isJump() && computed) kind = "computed";
            else if (ft.isJump() && ft.isConditional()) kind = "cbranch";
            else if (ft.isJump()) kind = "branch";
            else if (ft.isTerminal()) kind = "return";
            else kind = "fallthrough";
            if (computed && targets.isEmpty()) unresolved++;

            js.append(firstInsn ? "\n" : ",\n");
            firstInsn = false;
            js.append(q(rel(a))).append(":{\"len\":").append(len);
            js.append(",\"mnem\":").append(q(in.getMnemonicString().toLowerCase()));
            js.append(",\"ops\":").append(o);
            js.append(",\"flow\":{\"kind\":").append(q(kind));
            if (kind.equals("call") && computed) js.append(",\"computed\":true");
            js.append(",\"targets\":[");
            for (int i = 0; i < targets.size(); i++) js.append(i > 0 ? "," : "").append(q(targets.get(i)));
            js.append("]}");
            js.append(",\"relocs\":").append(rl).append('}');
        }
        js.append("\n},\n\"check\":{");
        js.append("\"instructions\":").append(seen.size());
        js.append(",\"functions\":").append(functions.size());
        js.append(",\"unresolved\":").append(unresolved);
        js.append(",\"relocs\":").append(relocsIn);
        js.append(",\"relocs_matched\":").append(relocsMatched);
        js.append(",\"pcrel\":").append(pcrel);
        js.append(",\"cleared_data\":").append(cleared);
        js.append(",\"unmatched\":").append(list(unmatched));
        js.append(",\"mismatched\":").append(list(mismatched));
        js.append(",\"unexplained\":").append(list(unexplained));
        js.append(",\"undecoded\":").append(list(undecoded));
        js.append("}}\n");
        try (PrintWriter w = new PrintWriter(args[2])) {
            w.print(js);
        }
        println(String.format("exported %d instructions, %d relocs (%d matched), %d unresolved, %d unexplained",
                seen.size(), relocsIn, relocsMatched, unresolved, unexplained.size()));
    }

    private int countPcrel = 0;

    private static FlowType rawFlow(Instruction in) {
        return in.getPrototype().getFlowType(in.getInstructionContext());
    }
    private int cleared = 0;

    /**
     * The Amiga loader marks every relocated long as a pointer. Where the loader's marks precede any
     * disassembly, the pointer sits inside an instruction's operand and Ghidra refuses to decode it
     * ("conflicting data"). Flow reached this address, so the bytes are code: clear the data inside
     * the one instruction they decode to (never an instruction) and decode again.
     */
    private Instruction clearAndDisassemble(Address a) throws Exception {
        PseudoInstruction p;
        try {
            p = new PseudoDisassembler(currentProgram).disassemble(a);
        } catch (Exception e) {
            return null;
        }
        if (p == null) return null;
        Address end = a.add(p.getLength() - 1);
        List<Data> in = new ArrayList<>();
        for (Address x = a; x.compareTo(end) <= 0; x = x.next()) {
            if (getInstructionContaining(x) != null) return null;
            Data d = getDataContaining(x);
            if (d != null && d.isDefined() && !in.contains(d)) in.add(d);
        }
        for (Data d : in) {
            removeData(d);
            cleared++;
        }
        disassemble(a);
        return getInstructionAt(a);
    }

    private static String list(List<String> l) {
        StringBuilder b = new StringBuilder("[");
        for (int i = 0; i < l.size(); i++) b.append(i > 0 ? "," : "").append(q(l.get(i)));
        return b.append(']').toString();
    }

    /** Targets Ghidra found for a computed jump or call (a jump table its analysis resolved). */
    private List<Address> computedTargets(Instruction in) {
        List<Address> out = new ArrayList<>();
        if (!rawFlow(in).isComputed()) return out;
        for (Reference r : in.getReferencesFrom()) {
            if (r.getReferenceType().isFlow() && r.getToAddress().isMemoryAddress()
                    && hunkOf(r.getToAddress().getOffset()) >= 0)
                out.add(r.getToAddress());
        }
        return out;
    }

    /**
     * One varnode as [space, offset, size]. A ram address or a relocated / PC-relative constant
     * inside a hunk is written "hN+$off"; anything outside every hunk stays a number.
     */
    private String varnode(Varnode v, Map<Long, String> relocValue, boolean hasPcOperand, boolean flowTarget,
                           int opIndex, int slot, StringBuilder rl, Set<Long> hit, long lo, long hi,
                           List<String> unexplained, long insn, int insnLen) {
        AddressSpace sp = v.getAddress().getAddressSpace();
        String space = sp.getName();
        long off = v.getOffset();
        String value = Long.toString(off);
        boolean isRam = sp.isMemorySpace() && !sp.isUniqueSpace() && !sp.isRegisterSpace();
        boolean isConst = sp.isConstantSpace();
        if ((isRam || (isConst && v.getSize() == 4)) && relocValue.containsKey(off)) {
            String target = relocValue.get(off);
            value = q(target);
            hit.add(off);
            if (rl.length() > 1) rl.append(',');
            rl.append("{\"in_op\":").append(opIndex).append(",\"slot\":").append(slot)
              .append(",\"target\":").append(q(target)).append('}');
        } else if ((isRam || (isConst && v.getSize() == 4)) && off >= lo && off < hi && hunkOf(off) >= 0) {
            value = q(rel(off));
            if (flowTarget) {
                // a direct branch or call target: PC-relative by construction
            } else if (isConst && off == insn + insnLen) {
                // the return address a bsr/jsr pushes: the next instruction
            } else if (hasPcOperand) {
                countPcrel++;
            } else {
                unexplained.add(rel(insn) + " " + space + " " + rel(off));
            }
        }
        return "[" + q(space) + "," + value + "," + v.getSize() + "]";
    }
}
