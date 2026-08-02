# Member and Method Lookup

For a value of a local nominal type, `value.member` searches in this order:

1. declared fields
2. methods attached by an inherent `impl`
3. methods attached by a trait `impl`

Method calls remove the explicit `self` receiver from the callable argument list.
A method declared with `self: &mut Self` requires a mutable place. `Self` is
replaced with the implementation target before body checking.

Unknown members produce `NOM001` and a nearby-name suggestion where possible.
Optional values must be handled before member access.
