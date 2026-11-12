(set-logic ALL)
(declare-datatype Standing ((Decide) (Gap) (Holds) (Opinion)))
(define-fun-rec cnt ((s (Seq Standing))) Int (ite (= (seq.len s) 0) 0 (+ (ite (= (seq.nth s 0) Decide) 1 0) (cnt (seq.extract s 1 (- (seq.len s) 1))))))
(declare-const s (Seq Standing))
; false: cnt s <= 1
(assert (not (<= (cnt s) 1)))
(check-sat)
(get-model)
