# 验证记录

2026-09-16，在 macOS 上使用 Rust 1.87.0 验证 `svg2vector 0.1.0`。

| 检查 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | 通过 |
| `cargo clippy --offline --locked --all-targets --all-features -- -D warnings` | 通过 |
| `cargo test --offline --locked` | 64 项通过：46 单元测试、16 CLI 测试、2 库集成测试 |
| `cargo build --release --offline --locked` | 通过 |
| Release CLI 实测 | 帮助、单文件、尺寸与画布、错误输入、覆盖保护均通过；PATH 置空后仍可运行 |
| 18 个外部图标 | 全部批量转换成功，生成 XML 均可由官方 `VdPreview` 渲染 |
| 独立代码复核 | 渐变默认值、局部边界变换、裁剪合并、过小尺寸四项问题均已修正并复核 |

渲染对照使用固定的 `sdk-common:31.13.1` 转换与预览工具，比较同尺寸 PNG 的预乘 RGBA 值（归一化到 `0..1`）。18 个图标的均方根误差范围为 `0.000121..0.002289`，并已检查并排预览。此结果说明样本接近，不能证明所有 SVG 都与官方实现或浏览器等价。

外部图标及 Java 预览工具不随项目分发，也不是运行依赖。项目包含可直接试用的 `examples/icon.svg`。未执行 Android 真机渲染，也未验证其他操作系统的发布包。已知近似和拒绝范围见[转换基线](upstream.md)。
