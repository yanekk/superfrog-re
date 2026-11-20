# Brief for `pir plan`: the tests in a Tart virtual Mac, nightly, screen tests included

Written 2026-11-20 for the planner. Slug: `test-vm`. **Not to start before the disk-space gate below.**
Read the root `CLAUDE.md` (Conventions, tests), [README.md](README.md) (test.sh, full.sh, smoke.sh),
`toolkit/crates/amiga-e2e/src/os.rs` and `toolkit/crates/amiga-oracle/README.md` first.

## The goal

Every night the whole test set runs on a clean virtual Mac, not on the owner's own session, and that
includes the screen tests: the ones that open the real app, type into it, click its menus and capture its
window. In the morning there is one report: what passed, what failed, how long each took. The owner's Mac
can be locked or in use and the result does not depend on it; it must stay awake for the night, since
the guest runs on it.

## Why

- `tools/full.sh` (the nightly set) skips the screen tests unless `SF_E2E_OS=1` / `FD_E2E_OS=1` is set
  (`tools/test_full_command.py`; Superfrog alone has 32). They need a logged-in screen and two macOS
  permissions, Accessibility and Screen & System Audio Recording, granted to the app running them
  (`amiga_e2e::os_permissions`). A nightly run on the owner's session cannot count on either, so the
  tests that matter most to the faithful bar (the app end to end) never run at night.
- A test that passes only on the owner's Mac (its settings, caches, keyboard layout, a held permission)
  is evidence about that Mac. A clean machine made the same way each night removes that.

## Tart, as read on 2026-11-20

Tart (tart.run) runs macOS and Linux virtual machines on Apple Silicon through Apple's
Virtualization.framework. What matters here, each to be confirmed at the start of the plan (third-party
pages, not all from Tart's own docs):

- **Licence:** Fair Source; free on personal computers. Apple's macOS licence allows two macOS virtual
  machines at once per Mac, and Tart refuses a third.
- **Images:** ready macOS images from the project's registry (`ghcr.io/cirruslabs/macos-*`), about 25 GB
  to download, stored sparse under `~/.tart/` (`TART_HOME` moves it, for an external drive). The project
  appears under both `cirruslabs` and `openai` on GitHub; check which is current and pin a version.
- **Running:** `tart clone` a base image, `tart run` (with a window, or `--no-graphics`), commands through
  `tart exec` (a guest agent, run as a launchd global agent) or SSH, folders shared from the host
  (`--dir`, Virtio-FS).
- **Graphics:** the guest has Apple's paravirtualised GPU (Metal), which reports fewer features than the
  host's; some Metal programs are reported not to run. The apps draw through wgpu on Metal: whether they
  open, draw and capture in the guest is the plan's first unknown.

## The gate: disk space

The owner's Mac has 6.5 GB free of 228 GB (2026-11-20). The plan needs, at a guess: the base image
(about 25 GB, more once Xcode's command-line tools, Rust, Python, Java and Ghidra are in), the guest's
build directories (each game's `target/` is about 0.5 GB on the host, more for a full build with tests),
and room for a night's captures. Proposed bar: 80 GB free on the drive holding `TART_HOME`, an external
SSD allowed. The first task measures the real figure and the plan does not go on below it.

## Stages, in order of work

| Stage | Delivers | Proven by |
|---|---|---|
| Spike | One guest: Tart installed and pinned, a base image cloned, the toolchain in, the repo built, one screen test (`fd-e2e`'s shortest OS-layer test) passing in the guest | The test's output from inside the guest; the measured disk, memory and time |
| The image | A script that builds the test image from the base image, from nothing, every step written down: tools, the permissions granted, auto-login, sleep off, the keyboard layout | The script rebuilds the image on a clean Tart home and the spike's test passes in it |
| Secrets in, never baked | The disks, the Kickstart ROM and any key reach the guest at run time from the host (a read-only shared folder), never stored in the image | A check that the image holds none of them; the tests find them in the guest |
| The nightly run | A host-side command: start a fresh clone, run `tools/full.sh` with the screen tests on, copy the report and the captures out, delete the clone | One full night's run, its report next to the host's |
| Scheduling | The run wired to the nightly schedule in place of the host run, with a time limit and a clear failure when the guest does not come up | Two nights in a row, reported |
| Parity | The host's `full.sh` and the guest's, compared once: every difference explained | A list of the differences and why |

## Settled

- The nightly set is `tools/full.sh`; smoke stays on the host for pir (15 minutes, CLAUDE.md).
- Disks and ROMs are copyrighted: never committed, never in an image, never pushed to a registry.
- The image is rebuilt from a script, not kept as a hand-made machine.

## Open, for the planner to settle with the owner

1. **Where the image and clones live:** the internal disk after a clean-up, or an external SSD. Proposed:
   whichever gives the 80 GB, with `TART_HOME` set.
2. **How the permissions are granted in the guest.** The screen tests need Accessibility and Screen
   Recording for the terminal or agent that runs them. Options: grant them once by hand in the guest's
   window and keep the image (a person step, written down), or script them (needs SIP off in the guest,
   through its recovery mode). Proposed: once by hand, recorded, the rest scripted.
3. **The host's own nightly run:** keep it beside the guest's, or replace it. Proposed: replace it once
   parity is shown.
4. **Sound and pads in the guest.** The apps pace themselves on the audio device's clock; a scripted run
   plays nothing aloud. Proposed: run with the guest's audio device on and check the pacing tests hold.
   Pad tests stay on the host (no pad reaches a guest).
5. **Morning report:** a file in the repo's ignored `build/`, a page, or a message. Proposed: a file, and
   the owner says if more is wanted.
6. **Both guests at once:** Apple allows two; one per game halves the night. Proposed: one guest first.

## Out of scope

Running pir's workers in the guest, Linux guests, other machines or a cloud runner, a build cache shared
between host and guest (unless the spike shows a full build too slow for a night).
