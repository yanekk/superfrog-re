"""The signature scan on synthetic bytes, and unpacking checked three ways: a synthetic round trip, an
independent decoder against the game's routine on every real file, and docs/08-packers.md's table."""
import struct
import subprocess
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import unpack

CLEAN = bytes(range(256)) * 8
EFFICIENCY = b'\x09\x09\x09\x09'  # what every real ESL! file carries


def extracted():
    """build/files, extracted on first need. No zips means this fails naming where it looked."""
    if not unpack.MAIN_PROGRAM.exists():
        subprocess.run(['cargo', 'run', '-q', '-p', 'fd-tools', '--', 'extract', '--out', str(unpack.BUILD)],
                       cwd=unpack.GAME / 'engine', check=True, stdout=subprocess.DEVNULL)
    return unpack.BUILD / 'files'


class Bits:
    """Writes the format's stream: values MSB first, read from the file's end backwards."""
    def __init__(self):
        self.bits = []

    def put(self, value, count):
        self.bits += [(value >> i) & 1 for i in reversed(range(count))]

    def pack(self, size):
        skip = -len(self.bits) % 32
        bits = [0] * skip + self.bits
        longs = [sum(bit << i for i, bit in enumerate(bits[at:at + 32])) for at in range(0, len(bits), 32)]
        body = b''.join(v.to_bytes(4, 'big') for v in reversed(longs))
        return b'ESL!' + EFFICIENCY + body + size.to_bytes(3, 'big') + bytes([skip])


def literals(bits, text):
    """A literal run, last byte first since the format unpacks from the end."""
    bits.put(0, 1)
    left = len(text) - 1
    while left >= 3:
        bits.put(3, 2)
        left -= 3
    bits.put(left, 2)
    for byte in reversed(text):
        bits.put(byte, 8)


def stream(unread=0):
    """(text, packed) built token by token, in the order the unpacker meets them, with `unread` spare bits
    left at the stream's end."""
    bits = Bits()
    literals(bits, b'abcdefgh')                # 8 literals, needing the count's continuation
    bits.put(1, 2); bits.put(2, 9)             # length 3, offset 2: copies 'abc'
    literals(bits, b'XY')
    # A literal run is always followed by a match, with no token bit. Long form, 7 offset bits, offset 1.
    bits.put(3, 2); bits.put(0, 1); bits.put(1, 7)
    bits.put(7, 3); bits.put(2, 3)             # length 5 + 7 + 2 = 14, repeating 'XY'
    text = b'XY' * 7 + b'XY' + b'abc' + b'abcdefgh'
    bits.put(0, unread)
    return text, bits.pack(len(text))


def independent(data):
    """A decoder written from the format, not from the routine, to check the routine's outputs against."""
    efficiency, size, skip = data[4:8], int.from_bytes(data[-4:-1], 'big'), data[-1]
    stream_bytes, at, buf, left = data[8:-4], len(data) - 12, 0, 0
    out, o = bytearray(size), size

    def read(count):
        nonlocal at, buf, left
        value = 0
        for _ in range(count):
            if left == 0:
                at -= 1
                buf, left = stream_bytes[at], 8
            value, buf, left = value << 1 | buf & 1, buf >> 1, left - 1
        return value

    read(skip)
    while o:
        if read(1) == 0:
            run = 1
            while (x := read(2)) == 3:
                run += 3
            for _ in range(run + x):
                o -= 1
                out[o] = read(8)
            if not o:
                break
        x = read(2)
        width, run = efficiency[x], x + 2
        if x == 3:
            if read(1) == 0:
                width = 7
            offset = read(width)
            while (y := read(3)) == 7:
                run += 7
            run += y
        else:
            offset = read(width)
        for _ in range(run):
            o -= 1
            out[o] = out[o + 1 + offset]
    return bytes(out)


class Signatures(unittest.TestCase):
    def test_each_known_header_is_found(self):
        for magic in unpack.SIGNATURES:
            self.assertEqual(unpack.signature(magic + CLEAN), magic)

    def test_clean_data_is_not_misclassified(self):
        self.assertIsNone(unpack.signature(CLEAN))
        self.assertIsNone(unpack.signature(b''))

    def test_a_magic_inside_plain_data_does_not_count(self):
        self.assertIsNone(unpack.signature(CLEAN + b'PP20' + CLEAN))

    def test_an_executable_is_searched_only_in_its_stub_window(self):
        stub = unpack.HUNK_HEADER + bytes(0x1fc)
        self.assertEqual(unpack.signature(stub + b'S404' + CLEAN), b'S404')
        far = unpack.HUNK_HEADER + bytes(unpack.STUB_WINDOW)
        self.assertIsNone(unpack.signature(far + b'S404'))

    def test_entropy_tells_uniform_from_constant(self):
        self.assertEqual(unpack.entropy(bytes(64)), 0.0)
        self.assertEqual(unpack.entropy(bytes(range(256))), 8.0)

    def test_an_unpacked_or_unsupported_file_is_refused(self):
        with self.assertRaisesRegex(ValueError, 'not packed'):
            unpack.Unpacker().unpack(CLEAN)
        with self.assertRaisesRegex(ValueError, 'no unpacker'):
            unpack.Unpacker().unpack(b'RNC\x01' + CLEAN)


class RoundTrip(unittest.TestCase):
    def test_the_independent_decoder_reads_the_synthetic_stream(self):
        text, packed = stream()
        self.assertEqual(independent(packed), text)

    def test_the_games_routine_reads_the_synthetic_stream(self):
        extracted()
        text, packed = stream()
        self.assertEqual(unpack.Unpacker().unpack(packed), text)


class Damaged(unittest.TestCase):
    def setUp(self):
        extracted()
        self.packed = stream()[1]

    def test_a_malformed_file_is_refused_before_it_runs(self):
        p = self.packed
        for name, data in (('magic alone', b'ESL!'), ('no stream', b'ESL!' + EFFICIENCY),
                           ('not whole longs', p + b'\0'),
                           ('size beyond memory', p[:-4] + b'\xff\xff\xff' + p[-1:]),
                           ('size zero', p[:-4] + bytes(3) + p[-1:])):
            with self.subTest(name), self.assertRaisesRegex(ValueError, 'not a well-formed'):
                unpack.Unpacker().unpack(data)

    def test_a_stream_missing_its_first_long_is_refused(self):
        with self.assertRaisesRegex(ValueError, 'did not end where it starts'):
            unpack.Unpacker().unpack(self.packed[:8] + self.packed[12:])

    def test_a_stream_with_an_extra_first_long_is_refused(self):
        with self.assertRaisesRegex(ValueError, 'did not end where it starts'):
            unpack.Unpacker().unpack(self.packed[:8] + bytes(4) + self.packed[8:])


    def test_a_stream_ending_with_unread_bits_is_refused(self):
        unread = stream()[1][-1]  # exactly what the first long has spare, so A0 still ends right
        self.assertGreaterEqual(unread, 2)
        self.assertEqual(len(stream(unread)[1]), len(self.packed))
        with self.assertRaisesRegex(ValueError, 'did not end where it starts'):
            unpack.Unpacker().unpack(stream(unread)[1])


class RealDisks(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.files = sorted(p for p in extracted().rglob('*') if p.is_file())
        cls.packed = {p: unpack.signature(p.read_bytes()) for p in cls.files}

    def test_the_counts(self):
        found = list(self.packed.values())
        self.assertEqual(len(found), 61)
        self.assertEqual(found.count(b'ESL!'), 46)
        self.assertEqual(found.count(b'S404'), 1)
        self.assertEqual(found.count(None), 14)

    def test_the_games_routine_and_the_independent_decoder_agree_on_every_esl_file(self):
        unpacker = unpack.Unpacker()
        for path, magic in self.packed.items():
            if magic == b'ESL!':
                data = path.read_bytes()
                with self.subTest(path.name):
                    out = unpacker.unpack(data)
                    self.assertEqual(len(out), int.from_bytes(data[-4:-1], 'big'))
                    self.assertEqual(out, independent(data))

    def test_the_crack_program_unpacks_to_its_recorded_size_as_hunks(self):
        data = next(p for p, m in self.packed.items() if m == b'S404').read_bytes()
        out = unpack.Unpacker().unpack(data)
        self.assertEqual(len(out), 0x2a3bc)
        # The hunk list the stub then rebuilds: 7 hunks, its end marker, and the crack's own text.
        self.assertEqual(int.from_bytes(out[:4], 'big'), 6)
        self.assertEqual(out[-2:], b'\xff\xff')
        self.assertIn(b'CRACKED ON 06-11-96', out)
        # The sizes it lists account for all but the relocations between them.
        longs = sum(size & 0x3fffffff for size in struct.unpack_from('>7I', out, 4))
        self.assertLess(len(out) - (4 + 7 * 4 + longs * 4), 256)
        self.assertGreater(len(out) - (4 + 7 * 4 + longs * 4), 0)

    def test_the_doc_carries_the_current_table(self):
        doc = (unpack.GAME / 'docs' / '08-packers.md').read_text()
        self.assertIn(unpack.table(), doc)


if __name__ == '__main__':
    unittest.main()
