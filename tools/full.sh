#!/bin/sh
# The full set, for the owner's nightly schedule (smoke-tests DESIGN §2.1): everything in every project,
# the #[ignore]d tests included except the screen tests games/superfrog/test.sh leaves out.
PIR_TEST_SCOPE=all PIR_TEST_INCLUDE_IGNORED=1 exec "$(dirname "$0")/test.sh"
