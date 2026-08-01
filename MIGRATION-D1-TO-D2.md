# D1 to D2 Identity Migration

D1 used **Trion**, `trion`, and `.trn` as explicitly provisional identifiers.
D2 replaces them with:

| D1 provisional | D2 identity |
|---|---|
| Trion | Nivra |
| `trion` | `nivra` |
| `.trn` | `.nva` |
| `trion.toml` | `nivra.toml` |
| `trion.lock` | `nivra.lock` |

The D1 files remain in the repository as historical design evidence. Current D2
examples live under `examples/d2/` and use `.nva`. No source compatibility promise
exists yet because the compiler has not been released.
