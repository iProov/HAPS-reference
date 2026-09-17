/-
Proofs about the extracted `cmp_canonical`, the key comparison in HAPS's
RFC 8785 canonicalization.

Why this comparison matters: object members are ordered by it, so if it were not
a total order, member order — and therefore `intent_hash` and
`presentation_hash`, and therefore every signature verification — would depend
on the order keys happened to arrive in.

Everything below is stated about the *extracted* definitions in `Order.Funs`,
which Aeneas produced from `crates/haps-canon/src/order.rs`. Nothing is restated
by hand, and there are no `sorry`s.

## Scope, precisely

These are **step-level** theorems: they are about `cmp_canonical_loop.body`, one
iteration of the comparison, conditional on a successful final verdict
(`ok (done o)`). They do not establish that any step terminates or succeeds,
or prove whole-string ordering, UTF-8 decoding or canonicalization.

Lifting them to `cmp_canonical` itself requires induction over Aeneas's `loop`
combinator, which is defined by `partial_fixpoint`. That is the next step and is
deliberately not claimed here.
-/
import Order.Funs

open Aeneas Aeneas.Std Result ControlFlow

namespace haps_canon.order

/-- If one step returns a successful final verdict, swapping the arguments
returns the swapped verdict. Lifting this to the complete loop remains open. -/
theorem cmp_loop_body_antisymm
    (ua ub : alloc.vec.Vec Std.U16) (n i : Std.Usize) (o : Ordering)
    (h : cmp_canonical_loop.body ua ub n i = ok (done o)) :
    cmp_canonical_loop.body ub ua n i = ok (done o.swap) := by
  unfold cmp_canonical_loop.body at h ⊢
  split at h
  · -- i < n: compare the code units at position i.
    rename_i hlt
    simp only [hlt, if_true, reduceIte]
    cases hx : ua.index_usize i with
    | fail e => simp [hx] at h
    | div => simp [hx] at h
    | ok x =>
      cases hy : ub.index_usize i with
      | fail e => simp [hx, hy] at h
      | div => simp [hx, hy] at h
      | ok y =>
        by_cases hxy : (x : Nat) = (y : Nat)
        · -- equal units: the step continues rather than deciding, so `h` is absurd
          cases hi : i + 1#usize with
          | fail e => simp [hx, hy, hxy, hi] at h
          | div => simp [hx, hy, hxy, hi] at h
          | ok i1 => simp [hx, hy, hxy, hi] at h
        · by_cases hlt2 : (x : Nat) < (y : Nat)
          · -- x < y here, so y > x when swapped
            simp [hx, hy, hxy, hlt2] at h
            subst h
            have : ¬ (y : Nat) = (x : Nat) := fun hc => hxy hc.symm
            have : ¬ (y : Nat) < (x : Nat) := by omega
            simp [hx, hy, *, Ordering.swap]
          · -- x > y here, so y < x when swapped
            simp [hx, hy, hxy, hlt2] at h
            subst h
            have h1 : ¬ (y : Nat) = (x : Nat) := fun hc => hxy hc.symm
            have h2 : (y : Nat) < (x : Nat) := by omega
            simp [hx, hy, h1, h2, Ordering.swap]
  · -- i >= n: compare lengths.
    rename_i hge
    simp only [hge, if_false] at h ⊢
    by_cases hlen : ua.length = ub.length
    · simp [hlen] at h
      subst h
      simp [hlen.symm, Ordering.swap]
    · by_cases hlt : ua.length < ub.length
      · simp [hlen, hlt] at h
        subst h
        have h1 : ¬ ub.length = ua.length := fun hc => hlen hc.symm
        have h2 : ¬ ub.length < ua.length := by omega
        simp [h1, h2, Ordering.swap]
      · simp [hlen, hlt] at h
        subst h
        have h1 : ¬ ub.length = ua.length := fun hc => hlen hc.symm
        have h2 : ub.length < ua.length := by omega
        simp [h1, h2, Ordering.swap]

/-- If a step comparing a vector with itself returns a successful final
verdict, that verdict is equality. This does not establish successful termination. -/
theorem cmp_loop_body_irrefl
    (ua : alloc.vec.Vec Std.U16) (n i : Std.Usize) (o : Ordering)
    (h : cmp_canonical_loop.body ua ua n i = ok (done o)) :
    o = Ordering.eq := by
  unfold cmp_canonical_loop.body at h
  split at h
  · cases hx : ua.index_usize i with
    | fail e => simp [hx] at h
    | div => simp [hx] at h
    | ok x =>
      -- the units are identical, so the step continues; `h` says it decided
      cases hi : i + 1#usize with
      | fail e => simp [hx, hi] at h
      | div => simp [hx, hi] at h
      | ok i1 => simp [hx, hi] at h
  · simp at h
    exact h.symm

end haps_canon.order
