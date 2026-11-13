"""Writes the obligations of the node-semver port as SMT-LIB files, shaped as
Accord's compiler writes them (crates/accord/src/obligation): logic ALL, a
callee known only through its `requires` and `ensures` predicates, one script
per `ensures` clause asserting that `result` is the body's value and the
clause fails, each integer within its type's range, each list at most
4,294,967,295 elements, and each call's `requires` assumed where it runs.
`unsat` proves a script; `sat` refutes it.

Usage: [SHAPE=...] [FORMS=...] [CONTRACT=...] python gen.py <out folder>
  SHAPE: how the two forms of precedence are written: contract (default, the
    shape the port would use), negated, inline.
  FORMS: which forms the list comparator states: both (default), u, e.
  EQ: how equality is said: pointwise (default; the list comparator says "equal
    from k" element by element, a version's pre-release identifiers are equal
    as sequences); seq (the list comparator says the two tails are equal
    sequences); versions (as pointwise, and the versions' pre-release and
    build identifiers are equal element by element too).
  CONTRACT: the layout of the list comparators' `ensures`: both (default, the
    two forms of the owner decision), pin (the universal form and a clause
    that pins the first differing index), cases (one clause per case of the
    inputs: equal, one a prefix of the other, a first differing index),
    cases-pin (cases, and the pin), e (the existential form only, in every
    comparator), e-prefix (e, and the two cases of one list a strict prefix
    of the other).
  TRIGGERS: 0 (default) writes no `:pattern`; 1, 2 and 3 give every
    quantifier over a list index one of three designs (see add_triggers).
  SPEC: none (default), or recursive spec functions `fd` (the first index
    from k where two lists differ or one ends) and `canon_from` in place of
    the quantifiers; only with CONTRACT=e EQ=versions. b1: `define-fun-rec`;
    b2f1 and b2f2: an uninterpreted function with a fuel-limited defining
    axiom, fuel 1 and 2.
"""
import os
import re
import sys

MAX_LEN = 4294967295
I64 = "(and (<= (- 9223372036854775808) {e}) (<= {e} 9223372036854775807))"
I32 = "(and (<= (- 2147483648) {e}) (<= {e} 2147483647))"

PRELUDE_TEMPLATE = """(set-option :produce-models true)
(set-logic ALL)
(declare-datatypes ((PreId 0)) (((Numeric (digits String)) (Alpha (text String)))))
(declare-datatypes ((Version 0)) (((Ver (major Int) (minor Int) (patch Int) (pre (Seq PreId)) (build (Seq PreId))))))
; A numeric identifier is canonical, which the adapter guarantees: digits only
; and no leading zero, so length, then code point order, is numeric order.
(define-fun canon_id ((x PreId)) Bool
  (or ((_ is Alpha) x)
      (and (> (str.len (digits x)) 0)
           (str.in_re (digits x) (re.+ (re.range "0" "9")))
           (or (= (digits x) "0") (not (str.prefixof "0" (digits x)))))))
(define-fun canon_list ((a (Seq PreId))) Bool
  (forall ((i Int)) (=> (and (<= 0 i) (< i (seq.len a))) (canon_id (seq.nth a i)))))
; Numeric below alphanumeric; numeric by length then text; alphanumeric by
; text, which is code point order (`str.<`).
(define-fun id_lt ((x PreId) (y PreId)) Bool
  (or (and ((_ is Numeric) x) ((_ is Alpha) y))
      (and ((_ is Numeric) x) ((_ is Numeric) y)
           (or (< (str.len (digits x)) (str.len (digits y)))
               (and (= (str.len (digits x)) (str.len (digits y))) (str.< (digits x) (digits y)))))
      (and ((_ is Alpha) x) ((_ is Alpha) y) (str.< (text x) (text y)))))
(define-fun eq_from ((a (Seq PreId)) (b (Seq PreId)) (k Int)) Bool
  (and (= (seq.len a) (seq.len b))
       (forall ((j Int)) (=> (and (<= k j) (< j (seq.len a))) (= (seq.nth a j) (seq.nth b j))))))
(define-fun lt_at ((a (Seq PreId)) (b (Seq PreId)) (i Int)) Bool
  (or (and (= i (seq.len a)) (< i (seq.len b)))
      (and (< i (seq.len a)) (< i (seq.len b)) (id_lt (seq.nth a i) (seq.nth b i)))))
; "Greater" is neither less nor equal: cvc5 proves no order law of `str.<`.
(define-fun gt_at ((a (Seq PreId)) (b (Seq PreId)) (i Int)) Bool
  (or (and (= i (seq.len b)) (< i (seq.len a)))
      (and (< i (seq.len a)) (< i (seq.len b))
           (not (id_lt (seq.nth a i) (seq.nth b i))) (not (= (seq.nth a i) (seq.nth b i))))))
; The first index from k where the lists differ or one of them ends.
(define-fun first ((a (Seq PreId)) (b (Seq PreId)) (k Int) (i Int)) Bool
  (and (<= k i) (<= i (seq.len a)) (<= i (seq.len b))
       (forall ((j Int)) (=> (and (<= k j) (< j i)) (= (seq.nth a j) (seq.nth b j))))
       (or (= i (seq.len a)) (= i (seq.len b)) (not (= (seq.nth a i) (seq.nth b i))))))
{PRECEDENCE}(define-fun main_eq ((a Version) (b Version)) Bool
  (and (= (major a) (major b)) (= (minor a) (minor b)) (= (patch a) (patch b))))
(define-fun main_lt ((a Version) (b Version)) Bool
  (or (< (major a) (major b))
      (and (= (major a) (major b)) (< (minor a) (minor b)))
      (and (= (major a) (major b)) (= (minor a) (minor b)) (< (patch a) (patch b)))))
(define-fun main_gt ((a Version) (b Version)) Bool (main_lt b a))
; A version with a pre-release precedes the same version without one.
(define-fun pre_lt_u ((x (Seq PreId)) (y (Seq PreId))) Bool
  (or (and (> (seq.len x) 0) (= (seq.len y) 0))
      (and (> (seq.len x) 0) (> (seq.len y) 0) (lt_u x y 0))))
(define-fun pre_lt_e ((x (Seq PreId)) (y (Seq PreId))) Bool
  (or (and (> (seq.len x) 0) (= (seq.len y) 0))
      (and (> (seq.len x) 0) (> (seq.len y) 0) (lt_e x y 0))))
(define-fun pre_gt_u ((x (Seq PreId)) (y (Seq PreId))) Bool
  (or (and (= (seq.len x) 0) (> (seq.len y) 0))
      (and (> (seq.len x) 0) (> (seq.len y) 0) (gt_u x y 0))))
(define-fun pre_gt_e ((x (Seq PreId)) (y (Seq PreId))) Bool
  (or (and (= (seq.len x) 0) (> (seq.len y) 0))
      (and (> (seq.len x) 0) (> (seq.len y) 0) (gt_e x y 0))))
(define-fun ver_lt_u ((a Version) (b Version)) Bool (or (main_lt a b) (and (main_eq a b) (pre_lt_u (pre a) (pre b)))))
(define-fun ver_lt_e ((a Version) (b Version)) Bool (or (main_lt a b) (and (main_eq a b) (pre_lt_e (pre a) (pre b)))))
(define-fun ver_gt_u ((a Version) (b Version)) Bool (or (main_gt a b) (and (main_eq a b) (pre_gt_u (pre a) (pre b)))))
(define-fun ver_gt_e ((a Version) (b Version)) Bool (or (main_gt a b) (and (main_eq a b) (pre_gt_e (pre a) (pre b)))))
(define-fun prec_eq ((a Version) (b Version)) Bool (and (main_eq a b) {PREC_EQ}))
"""

EXTRA_PRELUDE = """; The first differing index exists.
(define-fun pinned ((a (Seq PreId)) (b (Seq PreId)) (k Int)) Bool
  (exists ((i Int)) (first a b k i)))
; x is a strict prefix of y from k.
(define-fun lprefix ((x (Seq PreId)) (y (Seq PreId)) (k Int)) Bool
  (and (< (seq.len x) (seq.len y))
       (forall ((j Int)) (=> (and (<= k j) (< j (seq.len x))) (= (seq.nth x j) (seq.nth y j))))))
"""

SHAPES = {
    # Both forms stated with `first`, the index where the lists first differ.
    "contract": '; Precedence of two lists from k, universal (at every such index) and existential.\n(define-fun lt_u ((a (Seq PreId)) (b (Seq PreId)) (k Int)) Bool\n  (forall ((i Int)) (=> (first a b k i) (lt_at a b i))))\n(define-fun gt_u ((a (Seq PreId)) (b (Seq PreId)) (k Int)) Bool\n  (forall ((i Int)) (=> (first a b k i) (gt_at a b i))))\n(define-fun lt_e ((a (Seq PreId)) (b (Seq PreId)) (k Int)) Bool\n  (exists ((i Int)) (and (first a b k i) (lt_at a b i))))\n(define-fun gt_e ((a (Seq PreId)) (b (Seq PreId)) (k Int)) Bool\n  (exists ((i Int)) (and (first a b k i) (gt_at a b i))))\n',
    "negated": '; Universal form by negation of the other two outcomes.\n(define-fun lt_e ((a (Seq PreId)) (b (Seq PreId)) (k Int)) Bool\n  (exists ((i Int)) (and (first a b k i) (lt_at a b i))))\n(define-fun gt_e ((a (Seq PreId)) (b (Seq PreId)) (k Int)) Bool\n  (exists ((i Int)) (and (first a b k i) (gt_at a b i))))\n(define-fun lt_u ((a (Seq PreId)) (b (Seq PreId)) (k Int)) Bool\n  (and (not (eq_from a b k)) (not (gt_e a b k))))\n(define-fun gt_u ((a (Seq PreId)) (b (Seq PreId)) (k Int)) Bool\n  (and (not (eq_from a b k)) (not (lt_e a b k))))\n',
    "inline": '; `first` written out in each form.\n(define-fun lt_e ((a (Seq PreId)) (b (Seq PreId)) (k Int)) Bool\n  (exists ((i Int)) (and (<= k i) (<= i (seq.len a)) (<= i (seq.len b))\n    (forall ((j Int)) (=> (and (<= k j) (< j i)) (= (seq.nth a j) (seq.nth b j))))\n    (lt_at a b i))))\n(define-fun gt_e ((a (Seq PreId)) (b (Seq PreId)) (k Int)) Bool\n  (exists ((i Int)) (and (<= k i) (<= i (seq.len a)) (<= i (seq.len b))\n    (forall ((j Int)) (=> (and (<= k j) (< j i)) (= (seq.nth a j) (seq.nth b j))))\n    (gt_at a b i))))\n(define-fun lt_u ((a (Seq PreId)) (b (Seq PreId)) (k Int)) Bool\n  (forall ((i Int)) (=> (and (<= k i) (<= i (seq.len a)) (<= i (seq.len b))\n    (forall ((j Int)) (=> (and (<= k j) (< j i)) (= (seq.nth a j) (seq.nth b j))))\n    (or (= i (seq.len a)) (= i (seq.len b)) (not (= (seq.nth a i) (seq.nth b i))))) (lt_at a b i))))\n(define-fun gt_u ((a (Seq PreId)) (b (Seq PreId)) (k Int)) Bool\n  (forall ((i Int)) (=> (and (<= k i) (<= i (seq.len a)) (<= i (seq.len b))\n    (forall ((j Int)) (=> (and (<= k j) (< j i)) (= (seq.nth a j) (seq.nth b j))))\n    (or (= i (seq.len a)) (= i (seq.len b)) (not (= (seq.nth a i) (seq.nth b i))))) (gt_at a b i))))\n',
}
SHAPE = os.environ.get("SHAPE", "contract")
# Which forms of precedence the list comparator's contract states: both, u or e.
FORMS = os.environ.get("FORMS", "both")
CONTRACT = os.environ.get("CONTRACT", "both")
EQ = os.environ.get("EQ", "pointwise")
PREC_EQ = {
    "pointwise": "(= (pre a) (pre b))",
    "seq": "(= (pre a) (pre b))",
    "versions": "(eq_from (pre a) (pre b) 0)",
}[EQ]
PRELUDE = PRELUDE_TEMPLATE.replace("{PRECEDENCE}", SHAPES[SHAPE]).replace("{PREC_EQ}", PREC_EQ)
if CONTRACT in ("pin", "cases", "cases-pin", "e-prefix"):
    PRELUDE += EXTRA_PRELUDE


def rng(expr, template=I64):
    return template.format(e=expr)


ID_BODY = """(ite ((_ is Numeric) x)
  (ite ((_ is Numeric) y)
    (ite (< (str.len (digits x)) (str.len (digits y))) (- 1)
      (ite (> (str.len (digits x)) (str.len (digits y))) 1
        (ite (str.< (digits x) (digits y)) (- 1) (ite (= (digits x) (digits y)) 0 1))))
    (- 1))
  (ite ((_ is Numeric) y) 1
    (ite (str.< (text x) (text y)) (- 1) (ite (= (text x) (text y)) 0 1))))"""

LIST_BODY = """(ite (= k (seq.len a)) (ite (= k (seq.len b)) 0 (- 1))
  (ite (= k (seq.len b)) 1
    (ite (not (= c!0 0)) c!0 c!1)))"""

# What "equal from k" says in the list comparator's `eq` clause: every element
# equal (pointwise), or the two tails equal as sequences (seq).
EQ_FROM = {
    "pointwise": "(eq_from a b k)",
    "versions": "(eq_from a b k)",
    "seq": "(= (seq.extract a k (- (seq.len a) k)) (seq.extract b k (- (seq.len b) k)))",
}[EQ]

BUILD_EQ = "(and (prec_eq a b) (eq_from (build a) (build b) 0))" if EQ == "versions" else "(= a b)"

LIST_ENSURES = {
    # The three-`ensures` contract of the recursive comparator, each result
    # with both forms of precedence (owner decision 3).
    "contract": [
        ("sign", "(and (<= (- 1) result) (<= result 1))"),
        ("eq", "(=> (= result 0) " + EQ_FROM + ")"),
        ("lt-u", "(=> (= result (- 1)) (lt_u a b k))"),
        ("lt-e", "(=> (= result (- 1)) (lt_e a b k))"),
        ("gt-u", "(=> (= result 1) (gt_u a b k))"),
        ("gt-e", "(=> (= result 1) (gt_e a b k))"),
    ],
}

SIGN = ("sign", "(and (<= (- 1) result) (<= result 1))")


def diff_cases(x, y, guard="true", k="0"):
    """The clauses of the cases layout for lists x and y from index k: they
    are equal, one is a strict prefix of the other, or there is a first
    differing index, and each case fixes the result."""
    def at(i, test):
        return f"(and {guard} (first {x} {y} {k} {i}) (< {i} (seq.len {x})) (< {i} (seq.len {y})) {test})"
    lt = f"(id_lt (seq.nth {x} i) (seq.nth {y} i))"
    return [
        ("same", f"(=> (and {guard} (eq_from {x} {y} {k})) (= result 0))"),
        ("short", f"(=> (and {guard} (lprefix {x} {y} {k})) (= result (- 1)))"),
        ("long", f"(=> (and {guard} (lprefix {y} {x} {k})) (= result 1))"),
        ("diff-lt", f"(forall ((i Int)) (=> {at('i', lt)} (= result (- 1))))"),
        ("diff-gt", f"(forall ((i Int)) (=> {at('i', '(not ' + lt + ')')} (= result 1)))"),
    ]


def pre_cases():
    """Versions: main numbers, then the pre-release identifiers (a version
    without a pre-release is above one with), in the cases layout."""
    la, lb = "(seq.len (pre a))", "(seq.len (pre b))"
    me = "(main_eq a b)"
    lt = "(id_lt (seq.nth (pre a) i) (seq.nth (pre b) i))"
    at = lambda test: f"(and {me} (first (pre a) (pre b) 0 i) (< i {la}) (< i {lb}) {test})"
    return [
        ("main-lt", "(=> (main_lt a b) (= result (- 1)))"),
        ("main-gt", "(=> (main_gt a b) (= result 1))"),
        ("pre-a-none", f"(=> (and {me} (= {la} 0) (> {lb} 0)) (= result 1))"),
        ("pre-b-none", f"(=> (and {me} (> {la} 0) (= {lb} 0)) (= result (- 1)))"),
        ("pre-short", f"(=> (and {me} (> {la} 0) (lprefix (pre a) (pre b) 0)) (= result (- 1)))"),
        ("pre-long", f"(=> (and {me} (> {lb} 0) (lprefix (pre b) (pre a) 0)) (= result 1))"),
        ("pre-diff-lt", f"(forall ((i Int)) (=> {at(lt)} (= result (- 1))))"),
        ("pre-diff-gt", f"(forall ((i Int)) (=> {at('(not ' + lt + ')')} (= result 1)))"),
    ]


PIN = [("pin", "(pinned a b k)")]
LIST_BY_CONTRACT = {
    "both": LIST_ENSURES["contract"],
    "pin": [SIGN, ("eq", "(=> (= result 0) " + EQ_FROM + ")")] + PIN
    + [c for c in LIST_ENSURES["contract"] if c[0] in ("lt-u", "gt-u")],
    "cases": [SIGN] + diff_cases("a", "b", k="k"),
}
LIST_BY_CONTRACT["cases-pin"] = LIST_BY_CONTRACT["cases"] + PIN
LIST_BY_CONTRACT["e"] = [c for c in LIST_BY_CONTRACT["both"] if not c[0].endswith("-u")]
# The existential form, and the two cases of one list a strict prefix of the other.
LIST_BY_CONTRACT["e-prefix"] = LIST_BY_CONTRACT["e"] + [c for c in LIST_BY_CONTRACT["cases"] if c[0] in ("short", "long")]

PRE_PIN = [("pin-pre", "(pinned (pre a) (pre b) 0)")]
BUILD_PIN = [("pin-build", "(pinned (build a) (build b) 0)")]
COMPARE_BY_CONTRACT = {
    "both": [
        SIGN,
        ("eq", "(=> (= result 0) (prec_eq a b))"),
        ("lt-u", "(=> (= result (- 1)) (ver_lt_u a b))"),
        ("lt-e", "(=> (= result (- 1)) (ver_lt_e a b))"),
        ("gt-u", "(=> (= result 1) (ver_gt_u a b))"),
        ("gt-e", "(=> (= result 1) (ver_gt_e a b))"),
    ],
}
COMPARE_BY_CONTRACT["e"] = [c for c in COMPARE_BY_CONTRACT["both"] if not c[0].endswith("-u")]
COMPARE_BY_CONTRACT["e-prefix"] = COMPARE_BY_CONTRACT["e"]
COMPARE_BY_CONTRACT["pin"] = [c for c in COMPARE_BY_CONTRACT["both"] if not c[0].endswith("-e")] + PRE_PIN
COMPARE_BY_CONTRACT["cases"] = [SIGN] + pre_cases() + [
    ("pre-same", "(=> (and (main_eq a b) (eq_from (pre a) (pre b) 0)) (= result 0))")]
COMPARE_BY_CONTRACT["cases-pin"] = COMPARE_BY_CONTRACT["cases"] + PRE_PIN
BUILD_BY_CONTRACT = {
    "both": [
        SIGN,
        ("eq", "(=> (= result 0) " + BUILD_EQ + ")"),
        ("lt-u", "(=> (= result (- 1)) (or (ver_lt_u a b) (and (prec_eq a b) (lt_u (build a) (build b) 0))))"),
        ("lt-e", "(=> (= result (- 1)) (or (ver_lt_e a b) (and (prec_eq a b) (lt_e (build a) (build b) 0))))"),
        ("gt-u", "(=> (= result 1) (or (ver_gt_u a b) (and (prec_eq a b) (gt_u (build a) (build b) 0))))"),
        ("gt-e", "(=> (= result 1) (or (ver_gt_e a b) (and (prec_eq a b) (gt_e (build a) (build b) 0))))"),
    ],
}
BUILD_BY_CONTRACT["e"] = [c for c in BUILD_BY_CONTRACT["both"] if not c[0].endswith("-u")]
BUILD_BY_CONTRACT["e-prefix"] = BUILD_BY_CONTRACT["e"]
BUILD_BY_CONTRACT["pin"] = [c for c in BUILD_BY_CONTRACT["both"] if not c[0].endswith("-e")] + PRE_PIN + BUILD_PIN
BUILD_BY_CONTRACT["cases"] = [SIGN] + pre_cases() + [
    ("build-" + n, t) for n, t in diff_cases("(build a)", "(build b)", "(prec_eq a b)")]
BUILD_BY_CONTRACT["cases-pin"] = BUILD_BY_CONTRACT["cases"] + PRE_PIN + BUILD_PIN

FUNCTIONS = {
    "cmp_id": dict(
        params=[("x", "PreId"), ("y", "PreId")],
        requires=["(canon_id x)", "(canon_id y)"],
        ensures=[
            ("sign", "(and (<= (- 1) result) (<= result 1))"),
            ("lt", "(=> (= result (- 1)) (id_lt x y))"),
            ("eq", "(=> (= result 0) (= x y))"),
            ("gt", "(=> (= result 1) (and (not (id_lt x y)) (not (= x y))))"),
        ],
        calls=[],
        body=ID_BODY,
    ),
    "cmp_list": dict(
        params=[("a", "(Seq PreId)"), ("b", "(Seq PreId)"), ("k", "Int")],
        requires=["(<= 0 k)", "(<= k (seq.len a))", "(<= k (seq.len b))", "(canon_list a)", "(canon_list b)"],
        ensures=[c for c in LIST_BY_CONTRACT[CONTRACT] if FORMS == "both" or not c[0].endswith("-" + ("e" if FORMS == "u" else "u"))],
        calls=[
            ("(and (< k (seq.len a)) (< k (seq.len b)))", "cmp_id", "(seq.nth a k) (seq.nth b k)"),
            ("(and (< k (seq.len a)) (< k (seq.len b)) (= c!0 0))", "cmp_list", "a b (+ k 1)"),
        ],
        body=LIST_BODY,
    ),
    "compare_main": dict(
        params=[("a", "Version"), ("b", "Version")],
        requires=[],
        ensures=[
            ("sign", "(and (<= (- 1) result) (<= result 1))"),
            ("eq", "(=> (= result 0) (main_eq a b))"),
            ("lt", "(=> (= result (- 1)) (main_lt a b))"),
            ("gt", "(=> (= result 1) (main_gt a b))"),
        ],
        calls=[],
        body="""(ite (< (major a) (major b)) (- 1) (ite (> (major a) (major b)) 1
  (ite (< (minor a) (minor b)) (- 1) (ite (> (minor a) (minor b)) 1
    (ite (< (patch a) (patch b)) (- 1) (ite (> (patch a) (patch b)) 1 0))))))""",
    ),
    "compare": dict(
        params=[("a", "Version"), ("b", "Version")],
        requires=["(canon_list (pre a))", "(canon_list (pre b))"],
        ensures=COMPARE_BY_CONTRACT[CONTRACT],
        calls=[
            ("true", "compare_main", "a b"),
            ("(and (= c!0 0) (> (seq.len (pre a)) 0) (> (seq.len (pre b)) 0))", "cmp_list", "(pre a) (pre b) 0"),
        ],
        body="""(ite (not (= c!0 0)) c!0
  (ite (= (seq.len (pre a)) 0) (ite (= (seq.len (pre b)) 0) 0 1)
    (ite (= (seq.len (pre b)) 0) (- 1) c!1)))""",
    ),
    "compare_build": dict(
        params=[("a", "Version"), ("b", "Version")],
        requires=["(canon_list (pre a))", "(canon_list (pre b))", "(canon_list (build a))", "(canon_list (build b))"],
        ensures=BUILD_BY_CONTRACT[CONTRACT],
        calls=[
            ("true", "compare", "a b"),
            ("(= c!0 0)", "cmp_list", "(build a) (build b) 0"),
        ],
        body="(ite (not (= c!0 0)) c!0 c!1)",
    ),
}

FAULTS = {
    # The shorter list is greater.
    "prefix": ("cmp_list", LIST_BODY
               .replace("(ite (= k (seq.len b)) 0 (- 1))", "(ite (= k (seq.len b)) 0 1)")
               .replace("(ite (= k (seq.len b)) 1\n", "(ite (= k (seq.len b)) (- 1)\n")),
    # A numeric tie returns 0 instead of going on to the next identifier.
    "skip": ("cmp_list", LIST_BODY.replace("(ite (not (= c!0 0)) c!0 c!1)", "c!0")),
    # A numeric identifier is above an alphanumeric one.
    "numeric": ("cmp_id", ID_BODY.replace("    (- 1))\n  (ite ((_ is Numeric) y) 1", "    1)\n  (ite ((_ is Numeric) y) (- 1)")),
}

TRIGGERS = int(os.environ.get("TRIGGERS", "0"))
SPEC = os.environ.get("SPEC", "none")
if SPEC != "none" and (CONTRACT, EQ) != ("e", "versions"):
    sys.exit("SPEC needs CONTRACT=e EQ=versions")


def balanced(text, start):
    """The index just after the S-expression that opens at text[start]."""
    depth, i, in_str = 0, start, False
    while i < len(text):
        c = text[i]
        if in_str:
            if c == '"':
                in_str = False
        elif c == '"':
            in_str = True
        elif c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    raise ValueError("unbalanced")


def drop_define(text, name):
    m = re.search(r"\(define-fun " + re.escape(name) + r" ", text)
    return text[:m.start()] + text[balanced(text, m.start()):].lstrip("\n")


SEQ = r"(\w+|\((?:pre|build) \w+\))"
QUANT = re.compile(r"\((forall|exists) \(\((\w+) Int\)\) ")


def add_triggers(text):
    """Gives each quantifier over one Int index a `:pattern` on the list
    elements it indexes. Design 1: the first list's element, `(seq.nth a i)`,
    as Dafny and Verus pick it. Design 2: each list's element as an
    alternative pattern. Design 3: the elements of both lists as one
    multi-pattern. A quantifier whose body names the index only through
    `first`, `lt_at` or `gt_at` gets its lists from that call."""
    out, pos = [], 0
    for m in QUANT.finditer(text):
        if m.start() < pos:
            continue
        end = balanced(text, m.start())
        var, body_start = m[2], m.end()
        body = text[body_start:end - 1]
        inner = add_triggers(body) if QUANT.search(body) else body
        seqs = []
        direct = re.findall(r"\(seq\.nth " + SEQ + " " + var + r"\)", body)
        calls = re.findall(r"\((?:first|lt_at|gt_at) " + SEQ + " " + SEQ + r" (?:[^()\s]+ )?" + var + r"\)", body)
        for x in direct + [x for pair in calls for x in pair]:
            if x not in seqs:
                seqs.append(x)
        if not seqs or inner.lstrip().startswith("(!"):
            out.append(text[pos:body_start] + inner + ")")
        else:
            nth = [f"(seq.nth {x} {var})" for x in seqs]
            if TRIGGERS == 1:
                pats = [nth[0]]
            elif TRIGGERS == 2:
                pats = nth
            else:
                pats = [" ".join(nth)]
            attrs = " ".join(f":pattern ({p})" for p in pats)
            out.append(text[pos:body_start] + f"(! {inner} {attrs}))")
        pos = end
    out.append(text[pos:])
    return "".join(out)


SEQ_SORT = "(Seq PreId)"
LIST_PARAMS = f"((a {SEQ_SORT}) (b {SEQ_SORT}) (k Int))"
FD_BODY = ("(ite (or (>= k (seq.len a)) (>= k (seq.len b))) k\n"
           "    (ite (= (seq.nth a k) (seq.nth b k)) {REC_FD} k))")
CANON_BODY = "(or (>= k (seq.len a)) (and (canon_id (seq.nth a k)) {REC_CANON}))"
SPEC_USERS = """(define-fun lt_e ((a (Seq PreId)) (b (Seq PreId)) (k Int)) Bool (lt_at a b (fd a b k)))
(define-fun gt_e ((a (Seq PreId)) (b (Seq PreId)) (k Int)) Bool (gt_at a b (fd a b k)))
(define-fun eq_from ((a (Seq PreId)) (b (Seq PreId)) (k Int)) Bool
  (and (= (fd a b k) (seq.len a)) (= (fd a b k) (seq.len b))))
"""


def spec_block():
    if SPEC == "b1":
        return ("; The first index from k where the lists differ or one of them ends, and whether the\n"
                "; identifiers from k on are canonical: recursive spec functions.\n"
                f"(define-fun-rec fd {LIST_PARAMS} Int\n  "
                + FD_BODY.format(REC_FD="(fd a b (+ k 1))") + ")\n"
                f"(define-fun-rec canon_from ((a {SEQ_SORT}) (k Int)) Bool\n  "
                + CANON_BODY.format(REC_CANON="(canon_from a (+ k 1))") + ")\n" + SPEC_USERS)
    fuel = {"b2f1": "(S F0)", "b2f2": "(S (S F0))"}[SPEC]
    return (
        "; The same spec functions as an uninterpreted function and a fuel-limited defining\n"
        "; axiom: it unfolds a call with fuel (S f) into one with fuel f, and the value\n"
        "; doesn't depend on the fuel.\n"
        "(declare-sort Fuel 0)\n(declare-const F0 Fuel)\n(declare-fun S (Fuel) Fuel)\n"
        f"(declare-fun fd_f (Fuel {SEQ_SORT} {SEQ_SORT} Int) Int)\n"
        f"(declare-fun canon_f (Fuel {SEQ_SORT} Int) Bool)\n"
        f"(assert (forall ((f Fuel) (a {SEQ_SORT}) (b {SEQ_SORT}) (k Int))\n"
        "  (! (= (fd_f (S f) a b k)\n  " + FD_BODY.format(REC_FD="(fd_f f a b (+ k 1))").replace("(seq.len a)", "(seq.len a)") + ")\n"
        "     :pattern ((fd_f (S f) a b k)))))\n"
        f"(assert (forall ((f Fuel) (a {SEQ_SORT}) (b {SEQ_SORT}) (k Int))\n"
        "  (! (= (fd_f f a b k) (fd_f F0 a b k)) :pattern ((fd_f f a b k)))))\n"
        f"(assert (forall ((f Fuel) (a {SEQ_SORT}) (k Int))\n"
        "  (! (= (canon_f (S f) a k)\n  " + CANON_BODY.format(REC_CANON="(canon_f f a (+ k 1))") + ")\n"
        "     :pattern ((canon_f (S f) a k)))))\n"
        f"(assert (forall ((f Fuel) (a {SEQ_SORT}) (k Int))\n"
        "  (! (= (canon_f f a k) (canon_f F0 a k)) :pattern ((canon_f f a k)))))\n"
        f"(define-fun fd {LIST_PARAMS} Int (fd_f {fuel} a b k))\n"
        f"(define-fun canon_from ((a {SEQ_SORT}) (k Int)) Bool (canon_f {fuel} a k))\n" + SPEC_USERS)


if SPEC != "none":
    PRELUDE = PRELUDE_TEMPLATE
    for name in ("canon_list", "eq_from", "first", "pre_lt_u", "pre_gt_u", "ver_lt_u", "ver_gt_u"):
        PRELUDE = drop_define(PRELUDE, name)
    PRELUDE = PRELUDE.replace("{PRECEDENCE}", spec_block()).replace("{PREC_EQ}", PREC_EQ)
    # The identifiers of a list are canonical from the index the callee starts at.
    for fn in FUNCTIONS.values():
        fn["requires"] = [
            re.sub(r"\(canon_list (\S+|\(\w+ \w+\))\)",
                   lambda m: f"(canon_from {m[1]} {'k' if m[1] in ('a', 'b') and 'k' in [p for p, _ in fn['params']] else '0'})",
                   r)
            for r in fn["requires"]]


def parameter_facts(function):
    out = ""
    for p, s in function["params"]:
        out += f"(declare-const {p} {s})\n"
        if s == "Int":
            out += f"(assert {rng(p)})\n"
        elif s == "(Seq PreId)":
            out += f"(assert (<= (seq.len {p}) {MAX_LEN}))\n"
        elif s == "Version":
            fields = " ".join(rng(f"({f} {p})") for f in ("major", "minor", "patch"))
            lengths = " ".join(f"(<= (seq.len ({f} {p})) {MAX_LEN})" for f in ("pre", "build"))
            out += f"(assert (and {fields} {lengths}))\n"
    return out


def declarations(names):
    """Each callee's `requires` and `ensures` as predicates, and the callee."""
    out = ""
    for name in names:
        f = FUNCTIONS[name]
        params = " ".join(f"({p} {s})" for p, s in f["params"])
        sorts = " ".join(s for _, s in f["params"])
        req = "(and " + " ".join(f["requires"]) + ")" if f["requires"] else "true"
        ens = "(and " + " ".join(t for _, t in f["ensures"]) + ")"
        out += f"(define-fun {name}.requires ({params}) Bool {req})\n"
        out += f"(define-fun {name}.ensures ({params} (result Int)) Bool {ens})\n"
        out += f"(declare-fun {name} ({sorts}) Int)\n"
    return out


def header(name, calls_run=True):
    """What every script of `name` starts with: the prelude, the callees'
    contracts, the parameters within their types, `requires`, and the calls
    with what is known of each."""
    f = FUNCTIONS[name]
    callees = sorted({c for _, c, _ in f["calls"]})
    out = PRELUDE + declarations(callees) + parameter_facts(f)
    for r in f["requires"]:
        out += f"(assert {r})\n"
    for n, (_, callee, args) in enumerate(f["calls"]):
        out += f"(declare-const c!{n} Int)\n(assert (= c!{n} ({callee} {args})))\n"
        out += f"(assert (=> ({callee}.requires {args}) ({callee}.ensures {args} c!{n})))\n"
        out += f"(assert {rng(f'c!{n}', I32)})\n"
    return out


def assumed_calls(name, upto=None):
    f = FUNCTIONS[name]
    return "".join(
        f"(assert (=> {guard} ({callee}.requires {args})))\n"
        for guard, callee, args in f["calls"][:upto]
    )


def claim_script(name, claim, body=None):
    f = FUNCTIONS[name]
    body = f["body"] if body is None else body
    return (
        header(name) + assumed_calls(name)
        + f"(declare-const result Int)\n(assert (and (= result {body}) (not {claim})))\n(check-sat)\n"
    )


def call_script(name, n):
    """That call `n` meets its callee's `requires` where it runs."""
    guard, callee, args = FUNCTIONS[name]["calls"][n]
    return header(name) + assumed_calls(name, n) + f"(assert (and {guard} (not ({callee}.requires {args}))))\n(check-sat)\n"


def measure_script(name, assertion):
    return header(name) + assertion + "\n(check-sat)\n"


def open_script(name):
    """An admitted input with two different results that both meet every
    `ensures`: `sat` means the contract leaves some result open."""
    f = FUNCTIONS[name]
    out = PRELUDE + declarations(sorted({c for _, c, _ in f["calls"]})) + parameter_facts(f)
    for r in f["requires"]:
        out += f"(assert {r})\n"
    contract = lambda r: "(and " + " ".join(t.replace("result", r) for _, t in f["ensures"]) + ")"
    return out + (
        "(declare-const r0 Int)\n(declare-const r1 Int)\n"
        f"(assert (and {contract('r0')} {contract('r1')} (not (= r0 r1))))\n(check-sat)\n"
    )


def main(folder):
    os.makedirs(folder, exist_ok=True)
    cases = {}
    for name, f in FUNCTIONS.items():
        for label, claim in f["ensures"]:
            cases[f"{name}-{label}-prove"] = claim_script(name, claim)
        cases[f"{name}-open-search"] = open_script(name)
        for n, (_, callee, _) in enumerate(f["calls"]):
            cases[f"{name}-call-{callee}-requires-prove"] = call_script(name, n)
    guard = FUNCTIONS["cmp_list"]["calls"][1][0]
    list_in_bounds = "(and (< k (seq.len a)) (< k (seq.len b)))"
    cases["cmp_list-bounds-a-prove"] = measure_script("cmp_list", f"(assert (and {list_in_bounds} (not (and (<= 0 k) (< k (seq.len a))))))")
    cases["cmp_list-bounds-b-prove"] = measure_script("cmp_list", f"(assert (and {list_in_bounds} (not (and (<= 0 k) (< k (seq.len b))))))")
    # `decreases length(a) - k`: smaller at the recursive call, and not negative.
    # The recursive call's `requires` is that obligation's other half.
    cases["cmp_list-decreases-prove"] = measure_script(
        "cmp_list",
        f"(assert (and {guard} (not (and (< (- (seq.len a) (+ k 1)) (- (seq.len a) k)) (>= (- (seq.len a) (+ k 1)) 0)))))")
    cases["cmp_list-overflow-k-plus-1-prove"] = measure_script("cmp_list", f"(assert (and {guard} (not {rng('(+ k 1)')})))")
    cases["cmp_list-overflow-len-minus-k-prove"] = measure_script("cmp_list", f"(assert (not {rng('(- (seq.len a) k)')}))")
    for fault, (name, body) in FAULTS.items():
        for label, claim in FUNCTIONS[name]["ensures"]:
            cases[f"{name}-{label}-fault-{fault}-refute"] = claim_script(name, claim, body)
    for case, text in cases.items():
        if TRIGGERS:
            text = add_triggers(text)
        with open(os.path.join(folder, f"{case}.smt2"), "w", newline="\n") as out:
            out.write(text)
    print(len(cases), "cases")


if __name__ == "__main__":
    main(sys.argv[1])
