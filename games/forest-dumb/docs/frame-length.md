# The frame length

Measured 2026-10-31 on the oracle (plan forest-dumb-start, T00), with `amiga_frame_lines` on every
stepped frame: 313 lines every frame from the crack intro on, checked to frame 14 000. The only
short frames are 75 frames of 312 lines, alternating over frames 677-825, under `Show`'s interlaced
picture. VPOSW is written `$8001` twice at frame 105 and `$8000` once at frame 1192.

So the game's timing carries `frame_lines: 313`, ready for `fd-core`; the toolkit takes it as a
parameter.
