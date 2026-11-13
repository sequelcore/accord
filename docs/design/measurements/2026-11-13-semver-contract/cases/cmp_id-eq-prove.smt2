(set-option :produce-models true)
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
; Precedence of two lists from k, universal (at every such index) and existential.
(define-fun lt_u ((a (Seq PreId)) (b (Seq PreId)) (k Int)) Bool
  (forall ((i Int)) (=> (first a b k i) (lt_at a b i))))
(define-fun gt_u ((a (Seq PreId)) (b (Seq PreId)) (k Int)) Bool
  (forall ((i Int)) (=> (first a b k i) (gt_at a b i))))
(define-fun lt_e ((a (Seq PreId)) (b (Seq PreId)) (k Int)) Bool
  (exists ((i Int)) (and (first a b k i) (lt_at a b i))))
(define-fun gt_e ((a (Seq PreId)) (b (Seq PreId)) (k Int)) Bool
  (exists ((i Int)) (and (first a b k i) (gt_at a b i))))
(define-fun main_eq ((a Version) (b Version)) Bool
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
(define-fun prec_eq ((a Version) (b Version)) Bool (and (main_eq a b) (= (pre a) (pre b))))
(declare-const x PreId)
(declare-const y PreId)
(assert (canon_id x))
(assert (canon_id y))
(declare-const result Int)
(assert (and (= result (ite ((_ is Numeric) x)
  (ite ((_ is Numeric) y)
    (ite (< (str.len (digits x)) (str.len (digits y))) (- 1)
      (ite (> (str.len (digits x)) (str.len (digits y))) 1
        (ite (str.< (digits x) (digits y)) (- 1) (ite (= (digits x) (digits y)) 0 1))))
    (- 1))
  (ite ((_ is Numeric) y) 1
    (ite (str.< (text x) (text y)) (- 1) (ite (= (text x) (text y)) 0 1))))) (not (=> (= result 0) (= x y)))))
(check-sat)
