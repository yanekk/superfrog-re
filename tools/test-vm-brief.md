# Brief for `pir plan`: screen and OS tests run in a Tart virtual Mac

Written 2026-11-20 for the planner. Slug: `test-vm`. **Not to start before the disk-space gate below.**
Read the root `CLAUDE.md` (Conventions, tests), [README.md](README.md) (test.sh, full.sh, smoke.sh),
`toolkit/crates/amiga-e2e/src/os.rs` and `toolkit/crates/amiga-oracle/README.md` first.

## The goal

Every test that needs a screen or the OS's input and window layer (the OS-layer tests: real key events,
real menu clicks, a capture of the real window) runs in a Tart virtual Mac, **whoever runs it**: a session
running its tests, pir's smoke at the end of a plan or a single run, and the nightly full set (owner,
2026-11-20). They never run on the owner's own screen. Everything else stays on the host as today.

The owner's Mac can be locked or in use and the result does not depend on it; it must stay awake while a
guest runs, since the guest runs on it.

## Why

- Today the OS-layer tests run only when `SF_E2E_OS=1` / `FD_E2E_OS=1` is set (`tools/test_full_command.py`;
  Superfrog alone has 32), on the owner's session: they need a logged-in screen and two macOS permissions,
  Accessibility and Screen & System Audio Recording, granted to the app running them
  (`amiga_e2e::os_permissions`). So the nightly run skips them, a session rarely runs them, and when one
  does it takes over the owner's screen and keyboard.
- A test that passes only on the owner's Mac (its settings, caches, keyboard layout, a held permission)
  is evidence about that Mac. A clean machine made the same way each time removes that.
- Run anywhere unattended, the app's end-to-end tests can become part of smoke and of a task's evidence,
  which the faithful bar wants (`knowledge/self-testing-faithful-bar.md`).

## Tart, as read on 2026-11-20

Tart (tart.run) runs macOS and Linux virtual machines on Apple Silicon through Apple's
Virtualization.framework. What matters here, each to be confirmed at the start of the plan (third-party
pages, not all from Tart's own docs):

- **Licence:** Fair Source; free on personal computers. Apple's macOS licence allows two macOS virtual
  machines at once per Mac, and Tart refuses a third.
- **Images:** ready macOS images from the project's registry (`ghcr.io/cirruslabs/macos-*`), about 25 GB
  to download, stored sparse under `~/.tart/` (`TART_HOME` moves it, for an external drive). The project
  appears under both `cirruslabs` and `openai` on GitHub; check which is current and pin a version.
- **Running:** `tart clone` a base image (copy-on-write, fast), `tart run` (with a window, or
  `--no-graphics`), commands through `tart exec` (a guest agent, run as a launchd global agent) or SSH,
  folders shared from the host (`--dir`, Virtio-FS).
- **Graphics:** the guest has Apple's paravirtualised GPU (Metal), which reports fewer features than the
  host's; some Metal programs are reported not to run. The apps draw through wgpu on Metal: whether they
  open, draw and capture in the guest is the plan's first unknown.

## The gate: disk space

The owner's Mac has 6.5 GB free of 228 GB (2026-11-20). The plan needs, at a guess: the base image
(about 25 GB, more once Xcode's command-line tools, Rust, Python, Java and Ghidra are in), up to two
running clones and their build directories (each game's `target/` is about 0.5 GB on the host, more for a
full test build), and room for captures. Proposed bar: 80 GB free on the drive holding `TART_HOME`, an
external SSD allowed. The first task measures the real figure and the plan does not go on below it.

## Stages, in order of work

| Stage | Delivers | Proven by |
|---|---|---|
| Spike | One guest: Tart installed and pinned, a base image cloned, the toolchain in, the repo built, one OS-layer test (`fd-e2e`'s shortest) passing in the guest | The test's output from inside the guest; the measured disk, memory, boot and build time |
| The image | A script that builds the test image from the base image, from nothing: tools, the permissions granted, auto-login, sleep off, the keyboard layout | The script rebuilds the image on a clean Tart home and the spike's test passes in it |
| Secrets in, never baked | The disks, the Kickstart ROM and any key reach the guest at run time from the host (a read-only shared folder), never stored in the image | A check that the image holds none of them; the tests find them in the guest |
| One way in | A host command (`tools/vm-test.sh` or as the planner names it) that runs a given test command on the caller's checkout in a guest and brings back its output, exit code and captures. The test selection routes every OS-layer test through it: `test.sh`, `full.sh`, `smoke.sh` and pir's test command, with nobody setting a variable | A session's test run, a smoke run and a full run each run an OS-layer test in a guest; a test that needs the screen run on the host refuses and names the command |
| Sharing the guests | At most two guests at once (Apple's limit) shared by pir's parallel workers, the night run and the owner's sessions: a queue, a time limit per run, a clear failure when a guest does not come up | Four workers asking at once: each run finishes or is refused with a reason, none hangs |
| Smoke and the budget | The crucial OS-layer tests labelled into smoke, the 15-minute budget re-measured with the guest's start-up cost | `smoke-times.tsv` with the guest runs in it; smoke within 15 minutes, or the owner asked |
| Nightly | `full.sh` with every OS-layer test, in a fresh guest, the report and captures copied out | Two nights in a row, reported |
| Parity | The host's results and the guest's compared once for the tests both can run: every difference explained | A list of the differences and why |

The planner orders the tasks; the spike goes first and decides whether the rest holds.

## Settled

- Screen and OS-layer tests run in a guest for every caller, never on the owner's screen (owner,
  2026-11-20). Tests that need neither stay on the host.
- Disks and ROMs are copyrighted: never committed, never in an image, never pushed to a registry.
- The image is rebuilt from a script, not kept as a hand-made machine.

## Open, for the planner to settle with the owner

1. **Where the image and clones live:** the internal disk after a clean-up, or an external SSD. Proposed:
   whichever gives the 80 GB, with `TART_HOME` set.
2. **How the permissions are granted in the guest.** Options: once by hand in the guest's window and kept
   in the image (a person step, written down), or scripted (needs SIP off in the guest, through its
   recovery mode). Proposed: once by hand, recorded, the rest scripted.
3. **A fresh guest a run, or a warm one.** A fresh clone each run is clean but pays boot and a cold build
   each time; a guest kept running with the checkout shared in and its build directory kept is fast but
   carries state. Proposed: warm guests for sessions and smoke, a fresh clone for the night, measured in
   the spike.
4. **Building in the guest or on the host.** The tests' binaries could be built on the host and copied in,
   both being macOS on Apple Silicon. Proposed: build in the guest (the clean-machine point), unless the
   spike shows it too slow for a session's run.
5. **When the guest is unavailable** (no disk, Tart missing, both guests busy past the limit). Proposed: the
   OS-layer tests fail with the reason, never fall back to the owner's screen.
6. **Sound and pads.** The apps pace themselves on the audio device's clock; a scripted run plays nothing
   aloud. Proposed: the guest's audio device on, and the pacing tests checked. Pad tests stay on the host
   (no pad reaches a guest).
7. **Morning report:** a file in the repo's ignored `build/`, a page, or a message. Proposed: a file.

## Out of scope

Running pir's workers themselves in a guest, Linux guests, other machines or a cloud runner.
