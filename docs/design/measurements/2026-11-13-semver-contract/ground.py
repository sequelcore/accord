"""Rewrites B2 cases (SPEC=b2f1 in gen.py) so that no axiom quantifies over a
sequence variable. Each spec function's unfolding and fuel-irrelevance axioms
are written once per sequence term of the obligation (`a`, `b`, or `(pre a)`,
`(pre b)`, `(build a)`, `(build b)`), quantified only over fuel and the index.
cvc5 1.4.2 with `--mbqi` recurses without bound on a quantifier over a `Seq`
variable (cvc5/cvc5#13161).

Usage: python ground.py <b2f1 folder> <output folder>
"""
import sys, os, glob
src, dst = sys.argv[1], sys.argv[2]
os.makedirs(dst, exist_ok=True)
FD = """(assert (forall ((f Fuel) (k Int))
  (! (= (fd_f (S f) {a} {b} k)
  (ite (or (>= k (seq.len {a})) (>= k (seq.len {b}))) k
    (ite (= (seq.nth {a} k) (seq.nth {b} k)) (fd_f f {a} {b} (+ k 1)) k)))
     :pattern ((fd_f (S f) {a} {b} k)))))
(assert (forall ((f Fuel) (k Int))
  (! (= (fd_f f {a} {b} k) (fd_f F0 {a} {b} k)) :pattern ((fd_f f {a} {b} k)))))
"""
CA = """(assert (forall ((f Fuel) (k Int))
  (! (= (canon_f (S f) {a} k)
  (or (>= k (seq.len {a})) (and (canon_id (seq.nth {a} k)) (canon_f f {a} (+ k 1)))))
     :pattern ((canon_f (S f) {a} k)))))
(assert (forall ((f Fuel) (k Int))
  (! (= (canon_f f {a} k) (canon_f F0 {a} k)) :pattern ((canon_f f {a} k)))))
"""
for p in glob.glob(os.path.join(src, "*.smt2")):
    t = open(p).read()
    i = t.index("(assert (forall ((f Fuel)"); j = t.index("(define-fun fd ")
    t = t[:i] + t[j:]
    if "(declare-const a (Seq PreId))" in t:
        pairs, seqs = [("a", "b")], ["a", "b"]
    elif "(declare-const a Version)" in t:
        pairs = [("(pre a)", "(pre b)"), ("(build a)", "(build b)")]
        seqs = ["(pre a)", "(pre b)", "(build a)", "(build b)"]
    else:
        pairs, seqs = [], []
    ax = "".join(FD.format(a=a, b=b) for a, b in pairs) + "".join(CA.format(a=a) for a in seqs)
    k = t.rindex("(check-sat)")
    open(os.path.join(dst, os.path.basename(p)), "w").write(t[:k] + ax + t[k:])
