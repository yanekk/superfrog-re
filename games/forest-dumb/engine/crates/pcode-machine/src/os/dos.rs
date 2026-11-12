//! dos.library: files read from the disk in df0 by `amiga-disk`'s OFS reader, each call finished at
//! once (owner, 2026-11-11: waiting for a pretend floppy buys the player nothing).

use amiga_disk::ofs::OfsImage;
use pcode_rt::{D0, Image, Regs, Stop};

use super::d;
use crate::Machine;
use crate::mem::ROM_BASE;

pub const OPEN: i32 = -30;
pub const CLOSE: i32 = -36;
pub const READ: i32 = -42;

/// `Open`'s mode for an existing file, read from its start: the only one answered, since nothing
/// here writes a disk.
pub const MODE_OLDFILE: u32 = 0x3ED;

/// Files open at once, at most: more is a program leaking handles.
pub const MAX_OPEN: usize = 16;

/// Where the made-up FileHandles would be: `Open` returns BPTRs to them, as AmigaDOS returns BPTRs
/// to its own. Nothing is there; the value is only handed back.
const HANDLES: u32 = ROM_BASE + 0x4000;

/// A file `Open` returned a handle to: its bytes, and how many have been read.
#[derive(Clone, Debug)]
pub(crate) struct OpenFile {
    data: Vec<u8>,
    pos: usize,
}

fn handle(slot: usize) -> u32 {
    (HANDLES + 0x40 * slot as u32) >> 2
}

/// The answer to dos' vector `lvo`, or `None` for one the machine does not answer.
pub(crate) fn call(
    m: &mut Machine,
    lvo: i32,
    r: &mut Regs,
    img: &Image,
    site: &str,
) -> Option<Result<(), Stop>> {
    Some(match lvo {
        OPEN => open(m, r, site),
        READ => read(m, r, img, site),
        CLOSE => close(m, r, site),
        _ => return None,
    })
}

/// The slot of the open file `h` names.
fn slot(m: &Machine, h: u32, call: &str, site: &str) -> Result<usize, Stop> {
    (0..m.os.files.len())
        .find(|&s| m.os.files[s].is_some() && handle(s) == h)
        .ok_or_else(|| Stop::Os(format!("{call} of handle ${h:08x}, not open, at {site}")))
}

/// `Open(d1 = name, d2 = mode)`: a handle in `d0`, or 0 for a file the disk lacks, as AmigaDOS
/// (whose `IoErr` is not kept: nothing measured asks it). The name may start with `df0:` or the
/// disk's volume name and a colon; any other device stops, since what it holds is unknown.
fn open(m: &mut Machine, r: &mut Regs, site: &str) -> Result<(), Stop> {
    let name = m.c_string(d(r, 1), "a file name", site)?;
    let name: String = name.iter().map(|&b| char::from(b)).collect();
    let mode = d(r, 2);
    if mode != MODE_OLDFILE {
        return Err(Stop::Os(format!(
            "Open of {name:?} in mode ${mode:x}, not MODE_OLDFILE, at {site}"
        )));
    }
    let disk = OfsImage::parse(&m.os.disks.df0).map_err(|e| {
        Stop::Os(format!(
            "Open of {name:?}: df0 holds no OFS disk ({e}), at {site}"
        ))
    })?;
    let path = match name.rsplit_once(':') {
        None => name.as_str(),
        Some((dev, path))
            if dev.eq_ignore_ascii_case("df0") || dev.eq_ignore_ascii_case(disk.volume_name()) =>
        {
            path
        }
        Some((dev, _)) => {
            return Err(Stop::Os(format!(
                "Open of {name:?}: device {dev:?} is not df0, at {site}"
            )));
        }
    };
    let Ok(data) = disk.read_file(path) else {
        m.os.opened.push((name, false));
        r.set(D0, 4, 0);
        return Ok(());
    };
    let free = match m.os.files.iter().position(Option::is_none) {
        Some(s) => s,
        None if m.os.files.len() < MAX_OPEN => {
            m.os.files.push(None);
            m.os.files.len() - 1
        }
        None => {
            return Err(Stop::Os(format!(
                "Open of {name:?}: {MAX_OPEN} files already open, at {site}"
            )));
        }
    };
    m.os.files[free] = Some(OpenFile { data, pos: 0 });
    m.os.opened.push((name, true));
    r.set(D0, 4, handle(free) as u64);
    Ok(())
}

/// `Read(d1 = handle, d2 = buffer, d3 = length)`: up to `length` bytes from where the last read
/// ended into RAM, and how many in `d0` (0 at the file's end). A buffer over translated code stops
/// as any store into it does; one outside RAM stops too.
fn read(m: &mut Machine, r: &mut Regs, img: &Image, site: &str) -> Result<(), Stop> {
    let (h, buf, len) = (d(r, 1), d(r, 2), d(r, 3));
    let s = slot(m, h, "Read", site)?;
    if (len as i32) < 0 {
        return Err(Stop::Os(format!(
            "Read of ${len:x} bytes, a negative length, at {site}"
        )));
    }
    let f = m.os.files[s].as_ref().expect("an open slot");
    let n = (len as usize).min(f.data.len() - f.pos);
    let bytes = f.data[f.pos..f.pos + n].to_vec();
    if let Some(at) = img.first_code_byte(buf, n as u32) {
        return Err(Stop::WriteIntoCode(img.describe(at)));
    }
    for (i, &b) in bytes.iter().enumerate() {
        let a = buf.wrapping_add(i as u32);
        if !m.ram_write(a, b) {
            return Err(Stop::Os(format!(
                "Read into ${:06x}, outside RAM, at {site}",
                a & 0xFF_FFFF
            )));
        }
    }
    m.os.files[s].as_mut().expect("an open slot").pos += n;
    r.set(D0, 4, n as u64);
    Ok(())
}

/// `Close(d1 = handle)`: the handle is free again; `d0` is left alone (1.3's Close returns
/// nothing).
fn close(m: &mut Machine, r: &mut Regs, site: &str) -> Result<(), Stop> {
    let s = slot(m, d(r, 1), "Close", site)?;
    m.os.files[s] = None;
    Ok(())
}
