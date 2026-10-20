# Finishing rules: superfrog-re

What pir does when a parallel build of this repo is finished and the person says go. Merge only.

The target branch is the branch the run was cut from. pir names it to the finisher; it is not set here.

1. In the person's main checkout, merge the build's branch into the target branch:
   `git -C <main checkout> merge pir/{slug}`, with the checkout on the target branch.
2. Confirm the target branch now contains the branch tip:
   `git -C <main checkout> merge-base --is-ancestor pir/{slug} <target>` succeeds.

Nothing else: no push, no pull request, no install.
