# Finishing rules: superfrog-re

What pir does when a parallel build of this repo is finished and the person says go. Keep the logs, then merge.

The target branch is the branch the run was cut from. pir names it to the finisher; it is not set here.

1. Keep the run's conversation logs, which pir deletes when the run is removed from its dashboard
   (owner, 2026-11-05, so tokens per activity can be measured after a run):
   `mkdir -p ~/.cache/superfrog-re/runs/{slug} && cp -R <main checkout>/plans/{slug}/.parallel/control/conversations ~/.cache/superfrog-re/runs/{slug}/`.
   A run with no `conversations/` folder skips this step.
2. In the person's main checkout, merge the build's branch into the target branch:
   `git -C <main checkout> merge pir/{slug}`, with the checkout on the target branch.
3. Confirm the target branch now contains the branch tip:
   `git -C <main checkout> merge-base --is-ancestor pir/{slug} <target>` succeeds.

Nothing else: no push, no pull request, no install, no deletion.
