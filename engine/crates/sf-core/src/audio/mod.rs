//! The game's sound driver (docs/11-sound.md): the Player 4.1A music replayer ([`music`]), the
//! sound-effect envelope player and the channel-3 sample streamer ([`sfx`]).
//!
//! The driver's state is the original's own variables, byte for byte: each block below is a run of
//! hunk-0 memory (`h0+$off`), read and written at the offsets the original uses, because the 68000
//! code overlays words and bytes freely (a row's last byte is also its empty-row counter, a volume
//! slide works on a word's low byte). That keeps every quirk, and makes transplanting a state from
//! the original and probing it back one copy each ([`AudioDriver::transplant`], [`AudioDriver::vars`]).
//!
//! Pointers held in that state are the original's absolute addresses. The driver reads what they
//! point at through [`Mem`]: the files the game loaded into chip RAM (a module, an effect bank),
//! with the bytes the driver itself changed there (the module header it relocates) laid over them.
//!
//! Every Paula write is appended to the caller's `out`, in the original's order. The positions are
//! left 0: the engine is not cycle-timed (DESIGN §2.1).

mod music;
mod sfx;
pub use sfx::SfxRest;

use std::collections::BTreeMap;

pub use sf_assets::sound::{BANK_ADDR, BankId, H0, STREAM_BUFFER, SoundData, TuneId};
pub use sf_paula::PaulaWrite;

/// The original's driver variables, as runs of hunk 0: `(h0 offset, length)`.
pub mod vars {
    /// `sfx_chan0..3` (`$12f62`, `$12f88`, `$12fbc`, `$12ff0`), `sfx_steal` (`$13024`) and the
    /// unused bytes after it that the shadow copy at a fade's end lands on (to `$13060`).
    pub const SFX: (u32, usize) = (0x12f62, 0xfe);
    /// `music_on` (`$13420`, a word) and the two bytes after it; `$13423` makes `F` speeds one slower.
    pub const MUSIC_ON: (u32, usize) = (0x13420, 4);
    /// The replayer: four channel records of `$2c` (`$144dc`), its globals (`$1458c`) and the
    /// four "channel free for music" bytes (`$145b2`, channel 3 first).
    pub const P41: (u32, usize) = (0x144dc, 0xda);
    /// The fade (`$146e0`: ceiling, speed, counter) and the shadow registers (`$146e4`, 16 bytes a
    /// channel: pointer, length, period, volume) the replayer writes instead of Paula.
    pub const SHADOW: (u32, usize) = (0x146e0, 0x44);
    /// The streamer: source pointer (`$148b0`), words left (`$148b4`, `$ffff` idle), volume (`$148b6`).
    pub const STREAM: (u32, usize) = (0x148b0, 8);
    /// `sfx_bank_init`'s tables: 256 sample pointers (`$1499e`), then 256 effect pointers (`$14d9e`).
    pub const SFX_TABLES: (u32, usize) = (0x1499e, 0x800);
    /// Where the module is loaded (`$9ec`, a long).
    pub const MODULE_ADDR: (u32, usize) = (0x9ec, 4);
    /// The module header as it was before the replayer relocated it (`$4814c`, 512 bytes).
    pub const HEADER_BACKUP: (u32, usize) = (0x4814c, 0x200);

    pub const ALL: [(u32, usize); 8] = [
        SFX,
        MUSIC_ON,
        P41,
        SHADOW,
        STREAM,
        SFX_TABLES,
        MODULE_ADDR,
        HEADER_BACKUP,
    ];
}

/// The files in chip RAM when the game is in `world` (1..6, 7 the bonus level) with its module at
/// `module_addr`: the world's effect bank and module; outside a level the title's module at the
/// bank's own place.
pub fn files_for(world: u32, module_addr: u32) -> Vec<(u32, File)> {
    let bank = if world <= 2 {
        BankId::Worlds12
    } else {
        BankId::Worlds36
    };
    match world {
        1..=6 => vec![
            (BANK_ADDR, File::Bank(bank)),
            (module_addr, File::Tune(TuneId::World(world as u8))),
        ],
        7 => vec![
            (BANK_ADDR, File::Bank(BankId::Bonus)),
            (module_addr, File::Tune(TuneId::Bonus)),
        ],
        _ => vec![(BANK_ADDR, File::Tune(TuneId::Title))],
    }
}

/// The bytes of the module header the replayer relocates in place, from the module's address.
pub const HEADER_LEN: u32 = 0x200;
/// Chip RAM from 0 that a zero instrument pointer makes the replayer read (`inst+6`, `inst+$a`).
pub const LOW_CHIP: usize = 12;

/// A run of the original's memory: big-endian reads and writes at its own offsets.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Block(pub(crate) Vec<u8>);

impl Block {
    pub(crate) fn b(&self, o: usize) -> u8 {
        self.0[o]
    }
    pub(crate) fn w(&self, o: usize) -> u16 {
        u16::from_be_bytes([self.0[o], self.0[o + 1]])
    }
    pub(crate) fn l(&self, o: usize) -> u32 {
        u32::from_be_bytes([self.0[o], self.0[o + 1], self.0[o + 2], self.0[o + 3]])
    }
    pub(crate) fn set_b(&mut self, o: usize, v: u8) {
        self.0[o] = v;
    }
    pub(crate) fn set_w(&mut self, o: usize, v: u16) {
        self.0[o..o + 2].copy_from_slice(&v.to_be_bytes());
    }
    pub(crate) fn set_l(&mut self, o: usize, v: u32) {
        self.0[o..o + 4].copy_from_slice(&v.to_be_bytes());
    }
}

/// A file the game put in chip RAM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum File {
    Tune(TuneId),
    Bank(BankId),
}

/// Chip RAM as the driver sees it: the files loaded into it (the latest wins where they overlap),
/// the bytes the driver wrote, and the first [`LOW_CHIP`] bytes.
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct Mem {
    files: Vec<(u32, File)>,
    written: BTreeMap<u32, u8>,
    low: [u8; LOW_CHIP],
    /// Runs of chip RAM given as they are (the story intro's: its hunk 2 holds the modules,
    /// samples and effects), read after the driver's own writes.
    image: Vec<(u32, Vec<u8>)>,
}

impl Mem {
    fn file_bytes(data: &SoundData, f: File) -> &[u8] {
        match f {
            File::Tune(t) => data.tune(t),
            File::Bank(b) => data.bank(b),
        }
    }

    pub(crate) fn b(&self, data: &SoundData, addr: u32) -> u8 {
        let addr = addr & 0xFF_FFFF;
        if let Some(&v) = self.written.get(&addr) {
            return v;
        }
        for (at, bytes) in &self.image {
            if addr >= *at && ((addr - at) as usize) < bytes.len() {
                return bytes[(addr - at) as usize];
            }
        }
        for &(at, f) in self.files.iter().rev() {
            let bytes = Mem::file_bytes(data, f);
            if addr >= at && ((addr - at) as usize) < bytes.len() {
                return bytes[(addr - at) as usize];
            }
        }
        self.low.get(addr as usize).copied().unwrap_or(0)
    }

    pub(crate) fn w(&self, data: &SoundData, addr: u32) -> u16 {
        u16::from_be_bytes([self.b(data, addr), self.b(data, addr.wrapping_add(1))])
    }

    pub(crate) fn l(&self, data: &SoundData, addr: u32) -> u32 {
        (self.w(data, addr) as u32) << 16 | self.w(data, addr.wrapping_add(2)) as u32
    }

    pub(crate) fn set_b(&mut self, addr: u32, v: u8) {
        self.written.insert(addr & 0xFF_FFFF, v);
    }

    pub(crate) fn set_l(&mut self, addr: u32, v: u32) {
        for (i, b) in v.to_be_bytes().into_iter().enumerate() {
            self.set_b(addr.wrapping_add(i as u32), b);
        }
    }

    /// The game loaded `f` at `addr`: what was there, and what the driver wrote there, is gone.
    fn load(&mut self, data: &SoundData, addr: u32, f: File) {
        let len = Mem::file_bytes(data, f).len() as u32;
        let end = addr + len;
        self.written.retain(|&a, _| a < addr || a >= end);
        self.files.retain(|&(at, g)| {
            let glen = Mem::file_bytes(data, g).len() as u32;
            !(at >= addr && at + glen <= end)
        });
        self.files.push((addr, f));
    }
}

/// Append one write.
pub(crate) fn wr(out: &mut Vec<PaulaWrite>, reg: u16, value: u16) {
    out.push(PaulaWrite {
        reg,
        value,
        hpos: 0,
        vpos: 0,
    });
}

/// A `clr.w` to a custom register, as the 68000 does it on the real machine (and the reference
/// emulator): two writes, the first of the word the CPU has prefetched (the next instruction's
/// first word, `prefetch`), then 0.
pub(crate) fn clr(out: &mut Vec<PaulaWrite>, reg: u16, prefetch: u16) {
    wr(out, reg, prefetch);
    wr(out, reg, 0);
}

pub(crate) const DMACON: u16 = 0x96;
pub(crate) const INTENA: u16 = 0x9a;

/// The sound driver: the original's driver variables and the chip RAM it reads.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AudioDriver {
    pub(crate) sfx: Block,
    pub(crate) music_on: Block,
    pub(crate) p41: Block,
    pub(crate) shadow: Block,
    pub(crate) stream: Block,
    pub(crate) tables: Block,
    pub(crate) module_addr: u32,
    pub(crate) backup: Block,
    pub(crate) mem: Mem,
    /// The story intro's copy of the driver (`d3_c04`), which differs from the game's in
    /// `music_stop` (h0+$391fc: music off and the audio DMA off, nothing more).
    pub(crate) intro: bool,
}

/// Where each variable block of [`vars::ALL`] is in the story intro's memory (its driver is the
/// game's code at other addresses, its variables in the same order around it). Its effect tables
/// are in hunk 2 and its module address and header backup do not exist (`None`).
pub struct IntroVars {
    pub sfx: u32,
    pub music_on: u32,
    pub p41: u32,
    pub shadow: u32,
    pub stream: u32,
    /// The sample pointers and the effect pointers.
    pub samples: u32,
    pub effects: u32,
}

impl AudioDriver {
    /// The story intro's driver as its memory holds it: `read(addr)` gives a byte, `chip` is the
    /// run of chip RAM the driver reads modules, samples and effects from.
    pub fn intro_from(
        read: &dyn Fn(u32) -> u8,
        at: &IntroVars,
        chip: (u32, Vec<u8>),
    ) -> AudioDriver {
        let block =
            |base: u32, len: usize| Block((0..len as u32).map(|i| read(base + i)).collect());
        let mut tables = block(at.samples, 0x400);
        tables.0.extend(block(at.effects, 0x400).0);
        let mut d = AudioDriver::blank();
        d.sfx = block(at.sfx, vars::SFX.1);
        d.music_on = block(at.music_on, vars::MUSIC_ON.1);
        d.p41 = block(at.p41, vars::P41.1);
        d.shadow = block(at.shadow, vars::SHADOW.1);
        d.stream = block(at.stream, vars::STREAM.1);
        d.tables = tables;
        d.mem.image = vec![chip];
        for i in 0..LOW_CHIP {
            d.mem.low[i] = read(i as u32);
        }
        d.intro = true;
        d
    }

    /// The story intro's variables to write back to its memory, `(address, bytes)`, and the
    /// bytes of chip RAM the driver wrote (a module it relocated).
    #[allow(clippy::type_complexity)]
    pub fn intro_vars(&self, at: &IntroVars) -> (Vec<(u32, Vec<u8>)>, Vec<(u32, u8)>) {
        (
            vec![
                (at.sfx, self.sfx.0.clone()),
                (at.music_on, self.music_on.0.clone()),
                (at.p41, self.p41.0.clone()),
                (at.shadow, self.shadow.0.clone()),
                (at.stream, self.stream.0.clone()),
            ],
            self.mem.written.iter().map(|(&a, &v)| (a, v)).collect(),
        )
    }

    /// Power-on: every variable as the executable holds it before the game runs.
    pub fn new(data: &SoundData) -> AudioDriver {
        let block = |(off, len): (u32, usize)| Block(data.h0_bytes(off, len));
        AudioDriver {
            sfx: block(vars::SFX),
            music_on: block(vars::MUSIC_ON),
            p41: block(vars::P41),
            shadow: block(vars::SHADOW),
            stream: block(vars::STREAM),
            tables: block(vars::SFX_TABLES),
            module_addr: Block(data.h0_bytes(vars::MODULE_ADDR.0, 4)).l(0),
            backup: block(vars::HEADER_BACKUP),
            mem: Mem::default(),
            intro: false,
        }
    }

    /// Every variable zero and no file loaded: what a [`crate::State`] holds before the game has
    /// set the driver up.
    pub fn blank() -> AudioDriver {
        let block = |(_, len): (u32, usize)| Block(vec![0; len]);
        AudioDriver {
            sfx: block(vars::SFX),
            music_on: block(vars::MUSIC_ON),
            p41: block(vars::P41),
            shadow: block(vars::SHADOW),
            stream: block(vars::STREAM),
            tables: block(vars::SFX_TABLES),
            module_addr: 0,
            backup: block(vars::HEADER_BACKUP),
            mem: Mem::default(),
            intro: false,
        }
    }

    /// Variable block `k` of [`vars::ALL`], as the original's bytes.
    pub fn var_block(&self, k: usize) -> Vec<u8> {
        match k {
            0 => self.sfx.0.clone(),
            1 => self.music_on.0.clone(),
            2 => self.p41.0.clone(),
            3 => self.shadow.0.clone(),
            4 => self.stream.0.clone(),
            5 => self.tables.0.clone(),
            6 => self.module_addr.to_be_bytes().to_vec(),
            _ => self.backup.0.clone(),
        }
    }

    /// Overwrite variable block `k` of [`vars::ALL`] with the original's bytes.
    pub fn set_var_block(&mut self, k: usize, b: &[u8]) {
        match k {
            0 => self.sfx.0 = b.to_vec(),
            1 => self.music_on.0 = b.to_vec(),
            2 => self.p41.0 = b.to_vec(),
            3 => self.shadow.0 = b.to_vec(),
            4 => self.stream.0 = b.to_vec(),
            5 => self.tables.0 = b.to_vec(),
            6 => self.module_addr = Block(b.to_vec()).l(0),
            _ => self.backup.0 = b.to_vec(),
        }
    }

    /// The first [`LOW_CHIP`] bytes of chip RAM, which a zero instrument pointer reads.
    pub fn set_low_chip(&mut self, low: [u8; LOW_CHIP]) {
        self.mem.low = low;
    }

    /// The files in chip RAM the driver reads, latest last.
    pub fn files(&self) -> &[(u32, File)] {
        &self.mem.files
    }

    /// A state taken from the original's memory (`read(addr)` gives the byte at a RAM address):
    /// every variable, the module header at the module's address and the first [`LOW_CHIP`] bytes of
    /// chip RAM. `files` are the files in chip RAM, loaded in that order (a bank at [`BANK_ADDR`],
    /// then the module at its address, say).
    pub fn transplant(
        data: &SoundData,
        read: &dyn Fn(u32) -> u8,
        files: &[(u32, File)],
    ) -> AudioDriver {
        let block =
            |(off, len): (u32, usize)| Block((0..len as u32).map(|i| read(H0 + off + i)).collect());
        let mut d = AudioDriver::new(data);
        d.sfx = block(vars::SFX);
        d.music_on = block(vars::MUSIC_ON);
        d.p41 = block(vars::P41);
        d.shadow = block(vars::SHADOW);
        d.stream = block(vars::STREAM);
        d.tables = block(vars::SFX_TABLES);
        d.module_addr = block(vars::MODULE_ADDR).l(0);
        d.backup = block(vars::HEADER_BACKUP);
        for &(at, f) in files {
            d.mem.load(data, at, f);
        }
        for i in 0..HEADER_LEN {
            let a = d.module_addr.wrapping_add(i);
            if d.mem.b(data, a) != read(a) {
                d.mem.set_b(a, read(a));
            }
        }
        for i in 0..LOW_CHIP {
            d.mem.low[i] = read(i as u32);
        }
        d
    }

    /// The modelled state at the original's addresses: each variable block (`H0 + off`), then the
    /// module header (at the module's address). What conformance compares with the original's RAM.
    pub fn vars(&self, data: &SoundData) -> Vec<(u32, Vec<u8>)> {
        let mut v = vec![
            (H0 + vars::SFX.0, self.sfx.0.clone()),
            (H0 + vars::MUSIC_ON.0, self.music_on.0.clone()),
            (H0 + vars::P41.0, self.p41.0.clone()),
            (H0 + vars::SHADOW.0, self.shadow.0.clone()),
            (H0 + vars::STREAM.0, self.stream.0.clone()),
            (H0 + vars::SFX_TABLES.0, self.tables.0.clone()),
            (
                H0 + vars::MODULE_ADDR.0,
                self.module_addr.to_be_bytes().to_vec(),
            ),
            (H0 + vars::HEADER_BACKUP.0, self.backup.0.clone()),
        ];
        let header = (0..HEADER_LEN)
            .map(|i| self.mem.b(data, self.module_addr.wrapping_add(i)))
            .collect();
        v.push((self.module_addr, header));
        v
    }

    // --- What the rest of the game does to the driver's world. ---

    /// The game's loader put `f` at `addr` in chip RAM (`load_file`, or a copy from a stash).
    pub fn file_loaded(&mut self, data: &SoundData, addr: u32, f: File) {
        self.mem.load(data, addr, f);
    }

    /// The game set the module address itself (`move.l #h3+$64c70,h0+$9ec` before the title music).
    pub fn set_module_addr(&mut self, addr: u32) {
        self.module_addr = addr;
    }

    pub fn module_addr(&self) -> u32 {
        self.module_addr
    }

    /// Whether music is playing (`music_on`).
    pub fn music_on(&self) -> bool {
        self.music_on.w(0) != 0
    }

    /// The streamer's source address, while a sample streams: the 160 bytes from there are what
    /// the frame copies to [`STREAM_BUFFER`] (for the mixer's view of chip RAM).
    pub fn stream_source(&self) -> Option<u32> {
        (self.stream.w(4) != 0xffff).then(|| self.stream.l(0))
    }

    // --- The interface the task names. ---

    /// Load `tune` at the module address and start it, as the game does before a world: the file
    /// lands at `module_addr` (set by [`AudioDriver::sfx_bank_init`] or
    /// [`AudioDriver::set_module_addr`]), then [`AudioDriver::music_start`].
    pub fn play_music(&mut self, data: &SoundData, tune: TuneId, out: &mut Vec<PaulaWrite>) {
        let at = self.module_addr;
        self.mem.load(data, at, File::Tune(tune));
        self.music_start(data, out);
    }

    /// [`AudioDriver::music_stop`].
    pub fn stop_music(&mut self, data: &SoundData, out: &mut Vec<PaulaWrite>) {
        self.music_stop(data, out);
    }

    /// `sfx_play`: effect `sfx` on `channel` 0..=3. The effect's own priority word decides whether
    /// it may replace one already playing there. The original returns at once in attract mode
    /// (`attract_mode`, `h0+$18`); a caller in attract mode does not call this.
    pub fn play_sfx(&mut self, data: &SoundData, sfx: u8, channel: u8, out: &mut Vec<PaulaWrite>) {
        assert!(channel < 4, "Paula has four channels");
        self.sfx_play(data, sfx, channel, out);
    }

    /// [`AudioDriver::play_sfx`] to its raster wait: interrupts off and on again around it, a
    /// vertical blank it held off coming before [`AudioDriver::play_sfx_rest`].
    pub fn play_sfx_begin(
        &mut self,
        data: &SoundData,
        sfx: u8,
        channel: u8,
        out: &mut Vec<PaulaWrite>,
    ) -> Option<SfxRest> {
        assert!(channel < 4, "Paula has four channels");
        self.sfx_play_begin(data, sfx, channel, out)
    }

    /// The rest of [`AudioDriver::play_sfx`] after its raster wait.
    pub fn play_sfx_rest(&mut self, data: &SoundData, rest: SfxRest, out: &mut Vec<PaulaWrite>) {
        self.sfx_play_rest(data, rest, out);
    }

    /// What the vertical-blank interrupt does for sound outside a level, in its order:
    /// [`AudioDriver::stream_tick`], [`AudioDriver::music_tick`], [`AudioDriver::music_dma_tick`],
    /// [`AudioDriver::sfx_tick`]. In a level the game's own code runs between the first two, so
    /// the level's frame calls them one by one.
    pub fn tick(&mut self, data: &SoundData, out: &mut Vec<PaulaWrite>) {
        self.stream_tick(data, out);
        self.music_tick(data, out);
        self.music_dma_tick(data, out);
        self.sfx_tick(data, out);
    }
}
