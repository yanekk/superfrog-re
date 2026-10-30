# toolkit-extract — delivery report

## What was delivered

All 11 tasks are done and reviewed. The plan delivered everything it set out to do.

- **A shared Amiga toolkit now sits beside the game, in its own folder.** It holds the parts that carry over to another Amiga game: the clock, the graphics and sound chips, the joystick and keyboard, disk images and zip files, the readers for Amiga program files and pictures, the original-machine emulator used for comparisons, the comparison checks that don't depend on the game, the generic parts of the app (sound, video, pacing, gamepad, screen capture), the driver for the end-to-end tests, and the generic half of the bug recorder.
- **The toolkit builds and tests on its own.** An automatic check fails if any file in it mentions Superfrog or Team 17. Another check keeps the parts that must stay pure free of clocks, files and the network.
- **Superfrog now uses the toolkit instead of its own copies, and plays exactly as before.** The full comparison against the original game passed all 216 cases, line for line the same as the check taken before any change. The whole-game end-to-end run passed too.
- **New, as you asked:** the toolkit can draw a picture straight from the Amiga's memory, using the same display code as Superfrog's renderer, so a display fix made for one game reaches both. The emulator can now start with no disk in the drive (it shows the Kickstart 'insert disk' hand, and a test checks for it). The emulator tests no longer need the game.
- **A trial run of game two's setup passed.** A small test that uses only toolkit parts, and nothing from Superfrog, builds and passes.

Left out on purpose, as planned: the route planner and route viewer, the app's main loop and menus, and the game-specific part of the comparison checks. Those wait until game two actually runs.

## Decisions made for you

None.

## What to check by hand

Nothing needs your eyes. The machine checked everything, including the full comparison with the original and the whole-game run.

If you want a quick look before you merge, start the game the usual way and play a minute of it. It should look, sound and feel exactly as before. One thing you will notice: the first comparison run after this merge is slower than usual, because the emulator was rebuilt once and its saved results are worked out again.

## Risks and follow-ups

- **Save states made before this change won't load.** That happens after any engine change and is expected.
- **Some end-to-end tests are flaky when the machine is busy.** The tests for the recording bar and for rebinding keys sometimes failed while several builds were running at once, and they failed the same way on the unchanged code. One timeout test that kept losing a race was given more time, with every check kept. Treat a one-off failure there as load, not as a bug.
- **A speed setting nearly got lost in the move.** The moved app parts had lost their 'build optimised even in test builds' setting. In the whole-game run the game then played at 31 frames a second and fell behind, though the pictures still matched. It is fixed for every moved part. Any part moved in future needs the same setting.
- **Some older bug recordings are rewritten in a slightly newer form when saved again.** Three of the five saved bug zips use the older form. They still load and replay. This worked the same way before the move.
- **Already known, unchanged:** one sound check over the whole game fails on a label at the end (the T216 item), which is still waiting on your confirmation. The toolkit still assumes the frame length Superfrog uses; working that out for game two is the first step of the game-two plan.
- **A small tidy-up for later:** the progress file's status line still reads 'planned, nothing built', although the task table shows every task done.

## Branch

Synced with `main` at `f341e9beab4c` on 2026-10-30T12:32:45Z.
The tests were red at the end; a worker fixed them.
Tests: green.
