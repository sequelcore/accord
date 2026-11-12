(set-logic ALL)
(declare-datatype Standing ((Decide) (Gap) (Holds) (Opinion)))
(declare-const s (Seq String))
; false claim: (all k: s[k] == V) implies len s <= 1 ; expect sat with a model
(assert (not (=> (forall ((k Int)) (=> (and (<= 0 k) (< k (seq.len s))) (= (seq.nth s k) "a"))) (<= (seq.len s) 1))))
(check-sat)
(get-model)
