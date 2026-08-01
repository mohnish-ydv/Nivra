# D1 to D2 Gate

D2 may begin only when:

1. GitHub Actions is green.
2. `bash verify.sh` prints the D1 golden-build marker.
3. The manual syntax tour has been reviewed.
4. The developer confirms that the pain map reflects the intended mission.
5. No D1 locked decision is being treated as deferred or vice versa.
6. The user reports `GG D1 Passed`.

D2 must not start compiler implementation. It is the final architecture/specification
delivery before implementation begins.
