"""pcode_export.py through Ghidra: hand-written programs, then the game, whole and the trial's set (full set).

Needs Ghidra and its JDK (translator DESIGN §5); without them the test fails naming the path, never
skips (§2.8).
"""
import io, json, os, sys, tempfile, unittest
from contextlib import redirect_stdout
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import pcode_export as P  # noqa: E402
from test_pcode_export import (ENTRIES, DEFECTS_FUNCS, DEFECTS_TAIL, MOVES_FUNCS, NORETURN_CALLS,  # noqa: E402
                               NORETURN_FUNCS, NORETURN_LOADER, defects_program, moves_program, noreturn_program,
                               two_hunk_program, whole_program)
import ghidra_headless as G  # noqa: E402

GAME = os.path.dirname(HERE)
DEFECTS_FIXTURE = os.path.join(GAME, 'engine', 'crates', 'fd-conform', 'tests', 'fixtures', 'defects.json')
MOVES_FIXTURE = os.path.join(os.path.dirname(DEFECTS_FIXTURE), 'moves.json')

ROUTINE_SEEDS = [(h, off, 'routine') for h, off, _ in ENTRIES]


class GhidraExport(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        exe = os.path.join(cls.tmp.name, 'p.exe')
        with open(exe, 'wb') as f:
            f.write(two_hunk_program())
        P.check_inputs(exe, P.SYMBOLS, P.ENTRIES)   # a missing Ghidra fails here, naming the path
        syms = [P.Sym(0, 0, 'start', 'func', 0, ''), P.Sym(0, 0x18, 'callee', 'func', 0, ''),
                P.Sym(0, 0x1c, 'other', 'func', 0, ''), P.Sym(0, 0x24, 'tail_a', 'func', 0, ''),
                P.Sym(0, 0x28, 'tail_b', 'func', 0, '')]
        with redirect_stdout(io.StringIO()):
            cls.doc = P.export(exe, syms, ROUTINE_SEEDS, os.path.join(cls.tmp.name, 'out'), name='p')
        cls.insns = cls.doc['insns']

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def flow(self, at):
        return self.insns[at]['flow']

    def test_every_instruction_by_direct_flow_hunk_relative(self):
        self.assertEqual(self.doc['entries'], [{'at': a, 'source': 'routine'}
                                               for a in ('h0+$0', 'h0+$1c', 'h0+$24', 'h0+$28', 'h0+$30')])
        self.assertEqual({i['hunk'] for i in self.insns.values()}, {0})
        self.assertEqual(sorted(self.insns, key=lambda a: int(a.split('$')[1], 16)),
                         ['h0+$0', 'h0+$6', 'h0+$a', 'h0+$c', 'h0+$e', 'h0+$12', 'h0+$14', 'h0+$16',
                          'h0+$18', 'h0+$1a', 'h0+$1c', 'h0+$20', 'h0+$24', 'h0+$28', 'h0+$2c', 'h0+$2e',
                          'h0+$30', 'h0+$36'])
        self.assertEqual([h['kind'] for h in self.doc['hunks']], ['CODE', 'DATA'])
        self.assertEqual(self.insns['h0+$0']['len'], 6)
        self.assertEqual(self.insns['h0+$6']['mnem'], 'bsr.w')

    def test_operands_and_bytes_for_the_census(self):
        # translator T04: the census reads the modes from these, not from the mnemonic
        i = self.insns['h0+$0']
        self.assertEqual(len(i['bytes']), 2 * i['len'])
        self.assertEqual(i['bytes'][:4], '41f9')   # lea (abs).l,a0
        self.assertEqual(len(i['operands']), 2)
        self.assertEqual(self.insns['h0+$6']['bytes'][:4], '6100')
        self.assertEqual(self.insns['h0+$16']['operands'], [])

    def test_the_lea_to_the_other_hunk_is_a_relocated_constant(self):
        i = self.insns['h0+$0']
        self.assertEqual(i['relocs'], [{'in_op': 0, 'slot': 0, 'target': 'h1+$10'}])
        self.assertEqual(i['ops'][0]['in'][0], ['const', 'h1+$10', 4])

    def test_flow_kinds_and_targets(self):
        self.assertEqual(self.flow('h0+$6'), {'kind': 'call', 'targets': ['h0+$18']})
        self.assertEqual(self.flow('h0+$e'), {'kind': 'cbranch', 'targets': ['h0+$c']})
        self.assertEqual(self.flow('h0+$a'), {'kind': 'fallthrough', 'targets': []})
        self.assertEqual(self.flow('h0+$16'), {'kind': 'return', 'targets': []})
        # the bsr's own P-code: its target and the return address it pushes, both hunk-relative
        ops = self.insns['h0+$6']['ops']
        self.assertEqual(ops[-1], {'op': 'CALL', 'out': None, 'in': [['ram', 'h0+$18', 4]]})
        self.assertIn(['const', 'h0+$a', 4], ops[1]['in'])

    def test_a_branch_into_a_shared_tail_is_a_branch(self):
        # Ghidra overrides such a branch into a tail call on the game (GameExport), not on this program
        for at in ('h0+$24', 'h0+$28'):
            self.assertEqual(self.flow(at), {'kind': 'branch', 'targets': ['h0+$2c']})
            self.assertEqual(self.insns[at]['ops'], [{'op': 'BRANCH', 'out': None, 'in': [['ram', 'h0+$2c', 4]]}])
            self.assertEqual(P.reach_counts(self.doc)[at], 3)

    def test_computed_jumps_ghidra_cannot_resolve_are_empty_and_counted(self):
        self.assertEqual(self.flow('h0+$14'), {'kind': 'call', 'computed': True, 'targets': []})
        self.assertEqual(self.flow('h0+$20'), {'kind': 'computed', 'targets': []})
        self.assertEqual(self.doc['check']['unresolved'], 2)

    def test_a_pc_relative_address_is_hunk_relative_and_explained(self):
        # Ghidra's P-code adds the displacement to the instruction's own address at run time
        self.assertEqual(self.insns['h0+$1c']['ops'][0]['in'], [['const', 'h0+$1c', 4], ['const', 2, 4]])
        self.assertEqual(self.insns['h0+$1c']['relocs'], [])
        c = self.doc['check']
        self.assertEqual(c['pcrel'], 1)
        self.assertEqual((c['unmatched'], c['mismatched'], c['undecoded']), ([], [], []))
        self.assertEqual((c['relocs'], c['relocs_matched'], c['relocs_in_file']), (1, 1, 2))

    def test_an_unrelocated_number_in_hunk_range_stays_a_number_and_is_reported(self):
        # Ghidra loads hunk 0 at $21f000; reading this as an address would be the guess DESIGN §7 rejects
        self.assertEqual(self.doc['ghidra_bases'][0], 0x21f000)
        self.assertEqual(self.insns['h0+$30']['ops'][0]['in'], [['const', 0x21f004, 4]])
        self.assertEqual(self.insns['h0+$30']['relocs'], [])
        self.assertEqual(self.doc['check']['unexplained'], ['h0+$30 const h0+$4'])


class WholeProgramExport(unittest.TestCase):
    """The made-up whole program seeded from its entry alone: the vector, the table and hunk 1 are found."""

    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        exe = os.path.join(cls.tmp.name, 'w.exe')
        with open(exe, 'wb') as f:
            f.write(whole_program())
        P.check_inputs(exe, P.SYMBOLS, P.ENTRIES)
        entries = os.path.join(cls.tmp.name, 'entries.toml')
        with open(entries, 'w') as f:
            f.write('')
        syms = [P.Sym(0, 0, 'start', 'func', 0, '')]
        with redirect_stdout(io.StringIO()):
            cls.doc = P.export(exe, syms, P.whole_seeds(syms, entries), os.path.join(cls.tmp.name, 'out'),
                               name='w')
        cls.insns = cls.doc['insns']

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def test_every_instruction_of_both_hunks_and_no_data(self):
        self.assertEqual(sorted(self.insns, key=P.parse_rel), [
            'h0+$0', 'h0+$8', 'h0+$e', 'h0+$12', 'h0+$14', 'h0+$16', 'h0+$1a', 'h0+$22', 'h0+$24', 'h0+$26',
            'h0+$28', 'h0+$2a', 'h0+$2c', 'h1+$0', 'h1+$2', 'h1+$4', 'h1+$6'])
        self.assertEqual({a: i['hunk'] for a, i in self.insns.items()},
                         {a: P.parse_rel(a)[0] for a in self.insns})
        self.assertEqual(P.summary(self.doc)['hunks'], [13, 4])

    def test_the_vector_install_seeds_its_handler_as_vector(self):
        self.assertEqual(self.doc['vectors'], ['h0+$2c'])
        self.assertEqual(self.doc['entries'], [{'at': 'h0+$0', 'source': 'program-entry'},
                                               {'at': 'h0+$2c', 'source': 'vector'}])
        self.assertEqual(self.insns['h0+$2c']['mnem'], 'rte')
        self.assertEqual(P.summary(self.doc)['by_source'], {'program-entry': 1, 'vector': 1})

    def test_the_jump_table_targets_are_found(self):
        self.assertEqual(self.insns['h0+$1a']['flow'], {'kind': 'computed', 'targets': ['h0+$24', 'h0+$28']})
        # the table base through (d8,PC,Xn) is PC-relative, not an unexplained number
        self.assertEqual(self.doc['check']['unexplained'], [])
        self.assertEqual(self.doc['check']['pcrel'], 1)

    def test_an_unresolved_computed_jump_is_listed_by_address(self):
        self.assertEqual(self.insns['h1+$4']['flow'], {'kind': 'computed', 'targets': []})
        self.assertEqual(self.doc['check']['unresolved_computed'], ['h1+$4'])
        self.assertEqual(P.summary(self.doc)['unresolved_computed'], 1)

    def test_every_relocation_lands_on_an_operand(self):
        c = self.doc['check']
        self.assertEqual((c['relocs'], c['relocs_matched'], c['undecoded'], c['outside']), (2, 2, [], []))


class NoReturnExport(unittest.TestCase):
    """Translator T43: a callee Ghidra judges non-returning still has every call's next instruction."""

    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        exe = os.path.join(cls.tmp.name, 'n.exe')
        with open(exe, 'wb') as f:
            f.write(noreturn_program())
        P.check_inputs(exe, P.SYMBOLS, P.ENTRIES)
        syms = [P.Sym(h, off, n, 'func', 0, '') for h, off, n in NORETURN_FUNCS]
        run_import = G.run_import

        def with_no_return(*a, **k):
            k['post_scripts'] = [('SetNoReturn.java', [NORETURN_LOADER])] + list(k['post_scripts'])
            return run_import(*a, **k)
        out = io.StringIO()
        with mock.patch.object(P.ghidra_headless, 'run_import', with_no_return), redirect_stdout(out):
            cls.doc = P.export(exe, syms, [(h, off, 'routine') for h, off, _ in NORETURN_FUNCS],
                               os.path.join(cls.tmp.name, 'out'), name='n')
        cls.ghidra = out.getvalue()
        cls.insns = cls.doc['insns']

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def test_ghidra_leaves_every_call_to_the_loader_without_its_fall_through(self):
        # Ghidra's own flow for each call, as on the game's 8 calls to load_file: no fall-through
        self.assertIn(f'no-return loader at {NORETURN_LOADER}', self.ghidra)
        self.assertEqual(self.doc['check']['no_return_calls'], NORETURN_CALLS)

    def test_every_call_keeps_its_next_instruction(self):
        for at in NORETURN_CALLS:
            self.assertEqual(self.insns[at]['flow'], {'kind': 'call', 'targets': ['h1+$20']})
        self.assertEqual(sorted(self.insns, key=P.parse_rel),
                         ['h0+$0', 'h1+$0', 'h1+$4', 'h1+$a', 'h1+$e', 'h1+$14', 'h1+$18', 'h1+$1e',
                          'h1+$20', 'h1+$22', 'h1+$24', 'h1+$26', 'h1+$28'])
        # the store after each call, its relocated address read from the pointer the loader marked
        self.assertEqual([[r['target'] for r in self.insns[a]['relocs']] for a in ('h1+$4', 'h1+$e', 'h1+$18')],
                         [['h2+$0'], ['h2+$4'], ['h2+$8']])
        self.assertEqual(P.reach_counts(self.doc)['h1+$0'], 12)


def export_defects(tmp):
    """The defects program through Ghidra, with its bra.w into `other` overridden to CALL_RETURN first,
    as Ghidra's function analysis does to such a branch on the game: (fixture document, Ghidra's output)."""
    exe = os.path.join(tmp, 'd.exe')
    with open(exe, 'wb') as f:
        f.write(defects_program())
    P.check_inputs(exe, P.SYMBOLS, P.ENTRIES)
    syms = [P.Sym(h, off, n, 'func', 0, '') for h, off, n in DEFECTS_FUNCS]
    run_import = G.run_import

    def with_override(*a, **k):
        k['post_scripts'] = [('SetFlowOverride.java', ['CALL_RETURN', DEFECTS_TAIL])] + list(k['post_scripts'])
        return run_import(*a, **k)
    out = io.StringIO()
    with mock.patch.object(P.ghidra_headless, 'run_import', with_override), redirect_stdout(out):
        doc = P.export(exe, syms, [(h, off, 'routine') for h, off, _ in DEFECTS_FUNCS], os.path.join(tmp, 'out'),
                       name='d')
    return {k: doc[k] for k in ('hunks', 'entries', 'insns')}, out.getvalue()


def export_moves(tmp):
    """The moves program (translator T10) through Ghidra: the fixture document."""
    exe = os.path.join(tmp, 'm.exe')
    with open(exe, 'wb') as f:
        f.write(moves_program())
    P.check_inputs(exe, P.SYMBOLS, P.ENTRIES)
    syms = [P.Sym(h, off, n, 'func', 0, '') for h, off, n in MOVES_FUNCS]
    with redirect_stdout(io.StringIO()):
        doc = P.export(exe, syms, [(h, off, 'routine') for h, off, _ in MOVES_FUNCS], os.path.join(tmp, 'out'),
                       name='m')
    return {k: doc[k] for k in ('hunks', 'entries', 'insns')}


def defects_text(doc):
    """The fixture's text: one instruction a line, so a change in Ghidra's P-code reads as a diff."""
    lines = ['{"hunks": ' + json.dumps(doc['hunks']) + ',', ' "entries": ' + json.dumps(doc['entries']) + ',',
             ' "insns": {']
    ats = sorted(doc['insns'], key=P.parse_rel)
    for k, at in enumerate(ats):
        lines.append(f'  {json.dumps(at)}: {json.dumps(doc["insns"][at])}' + (',' if k < len(ats) - 1 else ''))
    return '\n'.join(lines + [' }}']) + '\n'


class DefectsExport(unittest.TestCase):
    """Translator T07: Ghidra 12.0's defects on a made-up program, and the committed fixture
    (fd-conform's insn_defects.rs judges its translation on Moira) is what Ghidra exports today.
    Regenerate it with `python3 -I games/forest-dumb/tools/test_pcode_export_ghidra.py --write-defects`."""

    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.doc, cls.ghidra = export_defects(cls.tmp.name)
        cls.insns = cls.doc['insns']

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def test_the_committed_fixture_is_current(self):
        with open(DEFECTS_FIXTURE, encoding='utf-8') as f:
            self.assertEqual(f.read(), defects_text(self.doc), 'regenerate it: --write-defects')

    def test_a_bra_w_ghidra_marks_call_return_is_exported_as_a_branch(self):
        # Ghidra's own flow for it is now a call that ends the function; the export reads the
        # instruction's flow and the plain BRANCH P-code (ExportPcode.rawFlow, getPcode(false)).
        self.assertIn(f'override CALL_RETURN at {DEFECTS_TAIL}: bra.w', self.ghidra)
        self.assertIn('flow CALL_TERMINATOR', self.ghidra)
        self.assertEqual(self.insns[DEFECTS_TAIL]['flow'], {'kind': 'branch', 'targets': ['h0+$52']})
        self.assertEqual(self.insns[DEFECTS_TAIL]['ops'], [{'op': 'BRANCH', 'out': None, 'in': [['ram', 'h0+$52', 4]]}])

    def test_the_relocated_operand_names_its_hunk(self):
        self.assertEqual(self.insns['h0+$24']['relocs'][0], {'in_op': 0, 'slot': 0, 'target': 'h1+$4'})

    def test_every_instruction_is_exported(self):
        self.assertEqual(len(self.insns), 31)
        self.assertEqual({i['mnem'].split('.')[0] for i in self.insns.values()},
                         {'abcd', 'sbcd', 'nbcd', 'cmp', 'cmpa', 'cmpi', 'cmpm', 'bra', 'moveq', 'rts'})


class MovesExport(unittest.TestCase):
    """Translator T10: the move and control forms the game lacks, and the committed fixture
    (fd-conform's insn_moves.rs judges its translation on Moira) is what Ghidra exports today.
    Regenerate it with `python3 -I games/forest-dumb/tools/test_pcode_export_ghidra.py --write-moves`."""

    def test_the_committed_fixture_is_current_and_holds_every_instruction(self):
        with tempfile.TemporaryDirectory() as d:
            doc = export_moves(d)
        self.assertEqual(len(doc['insns']), 46)
        with open(MOVES_FIXTURE, encoding='utf-8') as f:
            self.assertEqual(f.read(), defects_text(doc), 'regenerate it: --write-moves')


class GameExport(unittest.TestCase):
    """The game's program from the extracted disk (`fd-tools extract`), the trial's routines only."""

    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        P.check_inputs(P.EXE, P.SYMBOLS, P.TRANSLATE)   # names the program's path when it is missing
        with redirect_stdout(io.StringIO()):
            cls.doc = P.export(P.EXE, P.load_symbols(P.SYMBOLS), P.routine_seeds(P.TRANSLATE),
                               os.path.join(cls.tmp.name, 'out'))

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def test_each_routine_reaches_what_the_inventory_counts(self):
        # Exact today; fails if a branch Ghidra calls a tail call is read as a call that falls
        # through, or if a pointer the loader marked inside an instruction stops its decoding.
        want = {P.rel(h, off): n for h, off, n in P.load_entries(P.TRANSLATE)}
        self.assertEqual(P.reach_counts(self.doc), want)

    def test_every_relocation_inside_the_code_lands_on_an_operand(self):
        c = self.doc['check']
        self.assertEqual(c['relocs'], c['relocs_matched'])
        self.assertEqual((c['unmatched'], c['mismatched'], c['unexplained'], c['undecoded']), ([], [], [], []))


class GameWholeExport(unittest.TestCase):
    """The game's whole program, as `pcode_export.py` writes it by default."""

    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        P.check_inputs(P.EXE, P.SYMBOLS, P.ENTRIES)
        syms = P.load_symbols(P.SYMBOLS)
        with redirect_stdout(io.StringIO()):
            cls.doc = P.export(P.EXE, syms, P.whole_seeds(syms, P.ENTRIES), os.path.join(cls.tmp.name, 'whole'))
            cls.trial = P.export(P.EXE, syms, P.routine_seeds(P.TRANSLATE), os.path.join(cls.tmp.name, 'trial'))

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def test_the_trial_routines_instructions_are_a_subset_of_the_whole_export(self):
        whole, trial = self.doc['insns'], self.trial['insns']
        self.assertEqual(sorted(set(trial) - set(whole), key=P.parse_rel), [])
        for a, i in trial.items():
            self.assertEqual((whole[a]['len'], whole[a]['mnem']), (i['len'], i['mnem']), a)

    def test_every_seed_is_an_entry_and_decodes(self):
        seeds = {P.rel(h, off) for h, off, _ in P.whole_seeds(P.load_symbols(P.SYMBOLS), P.ENTRIES)}
        entries = {e['at'] for e in self.doc['entries']}
        self.assertTrue(seeds <= entries)
        self.assertEqual(sorted(entries - set(self.doc['insns'])), [])
        c = self.doc['check']
        self.assertEqual((c['undecoded'], c['unmatched'], c['mismatched'], c['unexplained']), ([], [], [], []))
        self.assertEqual(c['relocs'], c['relocs_matched'])

    def test_the_installed_level_2_and_3_handlers_are_found_as_vectors(self):
        # entry h0+$100, $120 and system_off install level3_handler and level2_handler (docs/15)
        self.assertEqual(set(self.doc['vectors']), {'h0+$1eda', 'h0+$1f7a'})

    def test_hunk_4_the_music_player_is_exported(self):
        self.assertGreater(P.summary(self.doc)['hunks'][4], 0)

    def test_unresolved_computed_jumps_are_listed_by_address(self):
        c = self.doc['check']
        self.assertEqual(len(c['unresolved_computed']), c['unresolved'])
        computed_empty = sorted((a for a, i in self.doc['insns'].items()
                                 if i['flow'].get('computed') or i['flow']['kind'] == 'computed'
                                 if not i['flow']['targets']), key=P.parse_rel)
        self.assertEqual(sorted(c['unresolved_computed'], key=P.parse_rel), computed_empty)

    def test_every_call_keeps_its_next_instruction(self):
        # T43: Ghidra judges load_file (h0+$8ec4) non-returning; its 8 calls in hunk 4 still fall through
        missing = sorted((a for a, i in self.doc['insns'].items() if i['flow']['kind'] == 'call'
                          if P.rel(P.parse_rel(a)[0], P.parse_rel(a)[1] + i['len']) not in self.doc['insns']),
                         key=P.parse_rel)
        self.assertEqual(missing, [])

    def test_flows_out_of_the_program_come_only_from_unresolved_calls(self):
        # Ghidra aims an OS call `jsr -n(a6)` at the loader's stand-in library, outside every hunk
        c = self.doc['check']
        self.assertGreater(len(c['outside']), 0)
        self.assertEqual({o.split(' -> ')[0] for o in c['outside']} - set(c['unresolved_computed']), set())


if __name__ == '__main__':
    if sys.argv[1:] == ['--write-defects']:
        with tempfile.TemporaryDirectory() as d:
            text = defects_text(export_defects(d)[0])
        with open(DEFECTS_FIXTURE, 'w', encoding='utf-8') as f:
            f.write(text)
        print(f'wrote {DEFECTS_FIXTURE}')
    elif sys.argv[1:] == ['--write-moves']:
        with tempfile.TemporaryDirectory() as d:
            text = defects_text(export_moves(d))
        with open(MOVES_FIXTURE, 'w', encoding='utf-8') as f:
            f.write(text)
        print(f'wrote {MOVES_FIXTURE}')
    else:
        unittest.main()
