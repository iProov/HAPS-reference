import Lake
open Lake DSL

-- Aeneas ships the Lean library its extracted code imports (`import Aeneas`).
-- This local path does not enforce a revision or correspondence to extraction.
-- See docs/verification-toolchain.md for the recorded pin and reproduction limits.
require aeneas from "../../tools/aeneas/backends/lean"

package «hapsVerification»

-- Aeneas output, verbatim. `Order/Funs.lean` imports `Order.Types`, so the
-- module path has to match the llbc name.
@[default_target] lean_lib «Order» where
  globs := #[.andSubmodules `Order]

-- Hand-written proofs about that output.
@[default_target] lean_lib «Proofs» where
  globs := #[.andSubmodules `Proofs]
