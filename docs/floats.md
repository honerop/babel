# Native Float64 and recoverable conversions

The Rust backend accepts finite decimal/scientific literals such as `1.25`, `-0.5`, and `2e3`. They are inferred as Float64 (`f64` in generated Rust); integers remain signed 64-bit. The existing prefix operators `+`, `-`, `*`, `/`, `%`, `pow`, `min`, `max`, `neg`, `abs`, `clamp`, and numeric comparisons accept homogeneous Float64 arguments. Integer overflow checks are unchanged. Mixed integer and float arithmetic requires an explicit `intfloat` cast.

```babel
print + 1.25 2.75
square x = * x + x 0.0
print square 1.5
print + intfloat 2 0.5
```

Unconstrained numeric functions default to integer arithmetic, as before; float literals in their bodies specialize them to Float64. Generic numeric overloading is not implemented.

## JSON and conversion

- `boxfloat Float -> Data`: stores a finite Float64 JSON number.
- `asfloat Data -> Float`: extracts a finite JSON number and panics on malformed or wrong-type data, like existing `asint`.
- `tryfloat Data -> Result<Float,String>`: handles missing, wrong-type and out-of-range JSON numbers without panicking.
- `tryint`, `trytext`, `trybool`: equivalent recoverable typed conversions.
- `floatparse String -> Result<Float,String>`: parses a finite decimal.
- `floatint Float -> Result<Int,String>`: truncates toward zero and rejects non-finite or out-of-range values.
- `intfloat Int -> Float`: converts to f64, possibly losing precision for integers above 2^53.

```babel
data = json "{\\"opacity\\":0.65}"
opacity = result_get tryfloat field data "opacity" 1.0
print * opacity 100.0
```

Float arithmetic follows IEEE-754 semantics. NaN and infinity can result from some operations, but `boxfloat` rejects them because they cannot be represented in JSON.

Only the native Rust backend supports Float64. AIR/LLVM and the self-hosting C compiler are still integer-only.