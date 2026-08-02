# Implementation Coherence

An implementation is indexed by trait name and canonical target pattern. D8
rejects duplicate exact patterns with `TRT002`, validates every required method,
and compares receiver, parameter, and result types after replacing `Self`.

The package orphan rule allows an implementation when either the trait or target
nominal type is local. Implementing an imported trait for an imported type emits
`TRT006`.

D8 deliberately does not claim complete overlap reasoning for specialization.
Only exact canonical-pattern conflicts are stable in this delivery.
