# D7 to D8 Gate

D8 starts only after:

1. GitHub Actions `Verify D7 Nominal Members` is green.
2. `bash scripts/termux-verify.sh` prints the D7 golden-build marker.
3. All valid D7 fixtures pass.
4. Invalid fixtures produce NOM001 through NOM010 in order.
5. Record construction, method calls, enum variants, and JSON reports are manually checked.
6. The user reports `GG D7 Passed`.

D8 will focus on generic substitution, trait constraints, method selection, and
stronger nominal compatibility rather than starting code generation prematurely.
