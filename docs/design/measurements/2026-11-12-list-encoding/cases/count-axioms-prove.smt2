(set-logic ALL)
(declare-datatype Standing ((Decide) (Gap) (Holds) (Opinion)))
(declare-const s (Array Int Standing)) (declare-const n Int) (assert (>= n 0))
(declare-const cnt Int)
(define-fun anyd () Bool (exists ((k Int)) (and (<= 0 k) (< k n) (= (select s k) Decide))))
(assert (and (<= 0 cnt) (<= cnt n) (= (= cnt 0) (not anyd))))
; decisions' ensures, from the axioms
(assert (not (and (<= 0 cnt) (<= cnt n) (= (= cnt 0) (not anyd)))))
(check-sat)
