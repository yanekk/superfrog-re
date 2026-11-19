//! A stop on screen (translator DESIGN §2.4, §2.10; T39): the translated program has stopped, so
//! the picture freezes on its last whole frame and a band over the picture's foot says how it
//! stopped, where its bug recording went and how to quit. Drawn into the frame's RGBA, not by
//! AppKit, so a capture holds exactly what the window shows and a test reads it back; the saved
//! screenshot is the frame without the band, which `fd-conform bug replay` holds its own last
//! picture against.
//!
//! The text is a 5×7 bitmap font of printable ASCII in 6×9 cells, one picture pixel a font pixel,
//! so at the smallest window (the picture at 1×) every line is [`COLS`] characters at most and
//! the band lies inside the picture.

/// A character cell: the glyph's 5×7 and a pixel of space right and two below.
const CELL_W: usize = 6;
const CELL_H: usize = 9;
/// Pixels between the band's edge and its text.
const PAD: usize = 4;

/// The narrowest picture the band is laid out for: the translated program's canvas.
const PICTURE_W: usize = fd_app::CANVAS.2 as usize;

/// Characters a line holds within the picture.
pub const COLS: usize = (PICTURE_W - 2 * PAD) / CELL_W;

/// What the band says: how the program stopped, then where the recording went, then how to quit,
/// each wrapped to [`COLS`].
pub fn lines(stop: &str, saved: &str) -> Vec<String> {
    let mut out = Vec::new();
    for l in [
        format!("Stopped: {stop}"),
        saved.to_string(),
        "The game cannot go on. Esc or Cmd-Q quits.".to_string(),
    ] {
        out.extend(wrap(&l, COLS));
    }
    out
}

/// `text` in lines of at most `cols` characters, broken at spaces where it can be.
fn wrap(text: &str, cols: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for word in text.split(' ').filter(|w| !w.is_empty()) {
        let mut word: Vec<char> = word.chars().collect();
        // A word longer than a line is cut where the line ends.
        while word.len() > cols {
            if !line.is_empty() {
                out.push(std::mem::take(&mut line));
            }
            out.push(word.drain(..cols).collect());
        }
        let n = line.chars().count();
        if n > 0 && n + 1 + word.len() > cols {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.extend(word);
    }
    if !line.is_empty() || out.is_empty() {
        out.push(line);
    }
    out
}

/// The band's rows in a picture `height` tall for `n` lines: its top and its height, at the
/// picture's foot.
pub fn band(height: usize, n: usize) -> (usize, usize) {
    let h = (n * CELL_H + 2 * PAD - 2).min(height);
    (height - h, h)
}

/// `lines` drawn over `rgba` (`size` wide and tall, 4 bytes a pixel): the band's pixels darkened
/// to a quarter, the text white.
pub fn draw(rgba: &mut [u8], size: (u32, u32), lines: &[String]) {
    let (w, h) = (size.0 as usize, size.1 as usize);
    let (top, bh) = band(h, lines.len());
    for y in top..top + bh {
        for p in rgba[y * w * 4..(y + 1) * w * 4].chunks_exact_mut(4) {
            for c in &mut p[..3] {
                *c /= 4;
            }
        }
    }
    for (row, line) in lines.iter().enumerate() {
        let y0 = top + PAD + row * CELL_H;
        for (col, ch) in line.chars().enumerate() {
            let x0 = PAD + col * CELL_W;
            for (gy, bits) in glyph(ch).iter().enumerate() {
                for gx in 0..5 {
                    let (x, y) = (x0 + gx, y0 + gy);
                    if bits >> (4 - gx) & 1 == 1 && x < w && y < h {
                        let i = (y * w + x) * 4;
                        rgba[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
                    }
                }
            }
        }
    }
}

/// `ch`'s seven rows, bit 4 the leftmost pixel; `?` for a character the font lacks.
fn glyph(ch: char) -> [u8; 7] {
    let find = |c: char| {
        GLYPHS.lines().find_map(|l| {
            let mut it = l.chars();
            (it.next() == Some(c)).then(|| {
                let mut rows = [0u8; 7];
                for (r, art) in l[2..].split(' ').enumerate().take(7) {
                    rows[r] = art.bytes().fold(0, |acc, b| acc << 1 | (b == b'#') as u8);
                }
                rows
            })
        })
    };
    find(ch).or_else(|| find('?')).expect("the font has `?`")
}

/// Printable ASCII, a glyph a line: the character, then its seven rows of five, `#` lit.
const GLYPHS: &str = r##"  ..... ..... ..... ..... ..... ..... .....
! ..#.. ..#.. ..#.. ..#.. ..#.. ..... ..#..
" .#.#. .#.#. ..... ..... ..... ..... .....
# .#.#. .#.#. ##### .#.#. ##### .#.#. .#.#.
$ ..#.. .#### #.#.. .###. ..#.# ####. ..#..
% ##... ##..# ...#. ..#.. .#... #..## ...##
& .##.. #..#. #.#.. .#... #.#.# #..#. .##.#
' ..#.. ..#.. ..... ..... ..... ..... .....
( ...#. ..#.. .#... .#... .#... ..#.. ...#.
) .#... ..#.. ...#. ...#. ...#. ..#.. .#...
* ..... ..#.. #.#.# .###. #.#.# ..#.. .....
+ ..... ..#.. ..#.. ##### ..#.. ..#.. .....
, ..... ..... ..... ..... .##.. ..#.. .#...
- ..... ..... ..... ##### ..... ..... .....
. ..... ..... ..... ..... ..... .##.. .##..
/ ..... ....# ...#. ..#.. .#... #.... .....
0 .###. #...# #..## #.#.# ##..# #...# .###.
1 ..#.. .##.. ..#.. ..#.. ..#.. ..#.. .###.
2 .###. #...# ....# ...#. ..#.. .#... #####
3 ##### ...#. ..#.. ...#. ....# #...# .###.
4 ...#. ..##. .#.#. #..#. ##### ...#. ...#.
5 ##### #.... ####. ....# ....# #...# .###.
6 ..##. .#... #.... ####. #...# #...# .###.
7 ##### ....# ...#. ..#.. .#... .#... .#...
8 .###. #...# #...# .###. #...# #...# .###.
9 .###. #...# #...# .#### ....# ...#. .##..
: ..... .##.. .##.. ..... .##.. .##.. .....
; ..... .##.. .##.. ..... .##.. ..#.. .#...
< ...#. ..#.. .#... #.... .#... ..#.. ...#.
= ..... ..... ##### ..... ##### ..... .....
> .#... ..#.. ...#. ....# ...#. ..#.. .#...
? .###. #...# ....# ...#. ..#.. ..... ..#..
@ .###. #...# ....# .##.# #.#.# #.#.# .###.
A .###. #...# #...# ##### #...# #...# #...#
B ####. #...# #...# ####. #...# #...# ####.
C .###. #...# #.... #.... #.... #...# .###.
D ###.. #..#. #...# #...# #...# #..#. ###..
E ##### #.... #.... ####. #.... #.... #####
F ##### #.... #.... ####. #.... #.... #....
G .###. #...# #.... #.### #...# #...# .####
H #...# #...# #...# ##### #...# #...# #...#
I .###. ..#.. ..#.. ..#.. ..#.. ..#.. .###.
J ..### ...#. ...#. ...#. ...#. #..#. .##..
K #...# #..#. #.#.. ##... #.#.. #..#. #...#
L #.... #.... #.... #.... #.... #.... #####
M #...# ##.## #.#.# #.#.# #...# #...# #...#
N #...# #...# ##..# #.#.# #..## #...# #...#
O .###. #...# #...# #...# #...# #...# .###.
P ####. #...# #...# ####. #.... #.... #....
Q .###. #...# #...# #...# #.#.# #..#. .##.#
R ####. #...# #...# ####. #.#.. #..#. #...#
S .#### #.... #.... .###. ....# ....# ####.
T ##### ..#.. ..#.. ..#.. ..#.. ..#.. ..#..
U #...# #...# #...# #...# #...# #...# .###.
V #...# #...# #...# #...# #...# .#.#. ..#..
W #...# #...# #...# #.#.# #.#.# #.#.# .#.#.
X #...# #...# .#.#. ..#.. .#.#. #...# #...#
Y #...# #...# .#.#. ..#.. ..#.. ..#.. ..#..
Z ##### ....# ...#. ..#.. .#... #.... #####
[ .###. .#... .#... .#... .#... .#... .###.
\ ..... #.... .#... ..#.. ...#. ....# .....
] .###. ...#. ...#. ...#. ...#. ...#. .###.
^ ..#.. .#.#. #...# ..... ..... ..... .....
_ ..... ..... ..... ..... ..... ..... #####
` .#... ..#.. ..... ..... ..... ..... .....
a ..... ..... .###. ....# .#### #...# .####
b #.... #.... #.##. ##..# #...# #...# ####.
c ..... ..... .###. #.... #.... #...# .###.
d ....# ....# .##.# #..## #...# #...# .####
e ..... ..... .###. #...# ##### #.... .###.
f ..##. .#..# .#... ###.. .#... .#... .#...
g ..... .#### #...# #...# .#### ....# .###.
h #.... #.... #.##. ##..# #...# #...# #...#
i ..#.. ..... .##.. ..#.. ..#.. ..#.. .###.
j ...#. ..... ..##. ...#. ...#. #..#. .##..
k #.... #.... #..#. #.#.. ##... #.#.. #..#.
l .##.. ..#.. ..#.. ..#.. ..#.. ..#.. .###.
m ..... ..... ##.#. #.#.# #.#.# #...# #...#
n ..... ..... #.##. ##..# #...# #...# #...#
o ..... ..... .###. #...# #...# #...# .###.
p ..... ..... ####. #...# ####. #.... #....
q ..... ..... .##.# #..## .#### ....# ....#
r ..... ..... #.##. ##..# #.... #.... #....
s ..... ..... .###. #.... .###. ....# ####.
t .#... .#... ###.. .#... .#... .#..# ..##.
u ..... ..... #...# #...# #...# #..## .##.#
v ..... ..... #...# #...# #...# .#.#. ..#..
w ..... ..... #...# #...# #.#.# #.#.# .#.#.
x ..... ..... #...# .#.#. ..#.. .#.#. #...#
y ..... ..... #...# #...# .#### ....# .###.
z ..... ..... ##### ...#. ..#.. .#... #####
{ ...#. ..#.. ..#.. .#... ..#.. ..#.. ...#.
| ..#.. ..#.. ..#.. ..#.. ..#.. ..#.. ..#..
} .#... ..#.. ..#.. ...#. ..#.. ..#.. .#...
~ ..... ..... .#... #.#.# ...#. ..... ....."##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_font_holds_every_printable_character_in_five_by_seven() {
        let lines: Vec<&str> = GLYPHS.lines().collect();
        assert_eq!(lines.len(), 95);
        for (k, l) in lines.iter().enumerate() {
            let c = (b' ' + k as u8) as char;
            assert!(l.starts_with(c), "line {k} is not {c:?}: {l}");
            let rows: Vec<&str> = l[2..].split(' ').collect();
            assert_eq!(rows.len(), 7, "{c:?}");
            assert!(rows.iter().all(|r| r.len() == 5 && r.bytes().all(|b| b == b'.' || b == b'#')), "{c:?}");
        }
        assert_eq!(glyph(' '), [0; 7]);
        assert_eq!(glyph('é'), glyph('?'));
        assert_eq!(glyph('I')[0], 0b01110);
    }

    #[test]
    fn every_line_fits_the_picture_at_its_smallest() {
        let long_op = "x".repeat(200);
        for stop in [
            "untranslated: h0+$1a2b4".to_string(),
            "unsupported: abcd.b -(a0),-(a1) at h0+$5462".into(),
            format!("os: {long_op} at h0+$10"),
            "hang (steps)".into(),
        ] {
            let ls = lines(&stop, "Bug recording saved as bug-power-on-2026-11-19-1430-12.zip");
            assert!(ls.iter().all(|l| l.chars().count() <= COLS), "{ls:#?}");
            assert!(ls[0].starts_with("Stopped: "));
            assert!(ls.last().unwrap().contains("Esc"));
            assert!(PAD + COLS * CELL_W <= PICTURE_W);
            let (top, h) = band(256, ls.len());
            assert!(top + h <= 256 && h >= ls.len() * CELL_H, "{} lines", ls.len());
        }
        assert_eq!(wrap("ab cd ef", 5), ["ab cd", "ef"]);
        assert_eq!(wrap("abcdefg", 3), ["abc", "def", "g"]);
        assert_eq!(wrap("", 3), [""]);
    }

    #[test]
    fn the_band_darkens_its_rows_and_the_text_is_white() {
        let (w, h) = (320u32, 256u32);
        let mut rgba = vec![200u8; (w * h * 4) as usize];
        let ls = lines("hang (steps)", "saved");
        draw(&mut rgba, (w, h), &ls);
        let (top, _) = band(h as usize, ls.len());
        let px = |x: usize, y: usize| &rgba[(y * w as usize + x) * 4..][..3];
        assert_eq!(px(0, top - 1), [200, 200, 200], "above the band");
        assert_eq!(px(w as usize - 1, top), [50, 50, 50], "the band");
        // `S` of "Stopped": its top row is .####.
        assert_eq!(px(PAD + 1, top + PAD), [255, 255, 255]);
        assert_eq!(px(PAD, top + PAD), [50, 50, 50]);
    }
}
