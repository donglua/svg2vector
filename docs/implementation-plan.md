# Rust SVG conversion CLI: completed implementation

Goal: a native CLI and reusable library that converts SVG icons to Android VectorDrawable XML without Java or Android SDK at runtime.

Baseline: Android sdk-common 32.4.0 Svg2Vector and its tree/path/gradient helpers, the latest stable Maven release audited on 2026-09-17. The latest public alpha sdk-common 32.5.0-alpha05 has identical `com/android/ide/common/vectordrawable` conversion logic for the files used by this project. The upstream source entry is https://cs.android.com/android-studio/platform/tools/base/+/mirror-goog-studio-main:sdk-common/src/main/java/com/android/ide/common/vectordrawable/Svg2Vector.java . Preserve AOSP attribution for ported algorithms.

1. Complete: native path/matrix, tree/style/reference, gradient and XML serialization modules, with explicit errors for unsupported features.
2. Complete: single-file, stdin and directory CLI conversion, optional dimensions/canvas, and safe output handling.
3. Complete: geometry, colors, gradients, references and negative cases; 18 external samples compared with the official renderer.
4. Complete: formatting, clippy, 64 tests, release build and actual CLI happy/error/help paths. Delivered as a standalone Cargo project.

Stable interface: convert(&str, &Options) -> Result<String, Error>. Output is Android vector XML. Default sizing preserves input dimensions. No automatic project resource deletion, no network access during conversion, no JVM subprocesses.

Completion evidence: focused Rust tests, CLI happy/error/help paths, official-reference render comparisons. Do not claim full parity with every upstream SVG feature unless covered.

See verification.md for the acceptance record and upstream.md for compatibility boundaries. Source and runtime CLI behavior are verified; no Android device acceptance or claim of complete SVG support is included.
