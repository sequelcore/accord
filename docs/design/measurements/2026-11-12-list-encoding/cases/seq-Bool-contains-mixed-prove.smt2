(set-logic ALL)
(declare-datatype Standing ((Decide) (Gap) (Holds) (Opinion)))
(declare-const s (Seq Bool))
; 'any' as exists; 'all' as forall; prove: (all k: s[k] != V) implies not (V in s) and len >= 0
(assert (not (=> (forall ((k Int)) (=> (and (<= 0 k) (< k (seq.len s))) (not (= (seq.nth s k) true))))
               (and (not (seq.contains s (seq.unit true))) (>= (seq.len s) 0)))))
(check-sat)
