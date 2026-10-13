# T54 — story-handler-end-feed

**Phase:** 2 · **Depends on:** T53; blocks T56 · **Weight:** medium

Split from T46. Owner, 2026-10-13: conformance feeds the original's handler end (and, T46, its
entry); normal play uses the engine's own estimate.

## Goal

A draw starts where the original's does: conformance gives where the original's vertical-blank
handler acknowledged its interrupt (`INTREQ = $20`, in the step's Paula writes with its beam),
`Game::set_handler_end`; the draw's simulation starts there with the handler's end
(`move.w #$20` prefetch, `movem.l (a7)+,d0-d7/a0-a6` `p`+31×`r`+`p`, `rte` `rrrpp`), the
`vbl_flag` wait it returns into, `draw_frames`' call (`pww nwwpp`, `draw_frame`'s set-up).

## Open

Which instruction of the wait (`tst.w`, `beq.b`) the interrupt came in on decides 13 cycles:
derive it from the fed entry, or check both against the oracle's DMACON $8400.

## Done when

- [ ] Draw ends (DMACON $400) land in the original's frame in every scene the scan reaches; the
  handler a draw holds off starts where the oracle's interrupt list says.
