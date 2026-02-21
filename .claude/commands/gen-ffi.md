Generate Swift FFI bindings by running:

```
make bindgen
```

This builds the `seycore` library in release mode with `--features uniffi` and runs `uniffi-bindgen` to produce Swift bindings in the `out/` directory.

Run this after adding or changing `#[uniffi::export]` annotations in `src/ffi.rs`. The generated files in `out/` can then be copied into the Xcode project as needed.

---

**UniFFI reference**: https://mozilla.github.io/uniffi-rs/latest/
