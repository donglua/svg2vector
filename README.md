# svg2vector

将 SVG 图标转换为 Android VectorDrawable XML 的 Rust 命令行工具，同时提供 Rust 库接口。运行时无需 Java、Android Studio 或 Android SDK。

转换流程参考 AOSP `Svg2Vector`，固定版本和模块对应见 [转换基线](docs/upstream.md)。

## 构建与安装

需要 Rust 1.87 或更新版本。

```sh
cargo build --release --locked
./target/release/svg2vector --help
./target/release/svg2vector examples/icon.svg -o /tmp/ic_example.xml

# 可选：安装到 Cargo 的 bin 目录
cargo install --path . --locked
```

## 使用

```sh
# 单文件
svg2vector icon.svg -o ic_icon.xml

# 目录批量转换：仅处理该目录下的 .svg 文件
svg2vector ./svg -o ./drawable

# 标准输入与标准输出
cat icon.svg | svg2vector - > ic_icon.xml

# 保持图形尺寸，在 24×24 画布中居中
svg2vector icon.svg --canvas 24x24 -o ic_icon.xml

# 调整 Android 固有尺寸，单边设置时保持宽高比
svg2vector icon.svg --width 24 -o ic_icon.xml

# 明确替换已有输出
svg2vector icon.svg -o ic_icon.xml --force
```

默认保留 SVG 尺寸和比例。`--canvas` 只改变画布和居中位移，不拉伸图形；小于原图的画布可能裁切。`--width`、`--height` 设置 Android 的 dp 尺寸，同时指定两者可改变显示比例。

输出文件名需要符合 Android 资源命名规则，例如 `ic_home.xml`。已有文件默认受保护，`--force` 也不会允许覆盖输入文件。批量转换在所有源文件转换及输出检查完成后写入；每个文件独立原子替换，磁盘或权限错误不保证整个目录事务回滚。

XML 写入标准输出，错误信息写入标准错误。成功退出码为 `0`，转换或文件错误为 `1`，命令行参数错误由 clap 返回 `2`。

## 支持范围

- 路径及矩形、圆、椭圆、直线、多边形等基础形状。
- 分组、平移、缩放、旋转、矩阵变换及非零 viewBox 原点。
- 填充、描边、透明度、`evenodd` 镂空、`currentColor`。
- 线性和径向渐变、色标、引用继承、渐变坐标与变换。
- 本地 `defs/use` 引用、裁剪路径、简单 CSS 选择器和内联样式。

当前不支持文本、嵌入位图、滤镜、图案填充、蒙版、动画、复杂 CSS 选择器、嵌套 SVG viewport 和 `use` 引用 `symbol` 等特性。裁剪路径只接受一个非空图形，包含 `g/use` 展开后的图形；该图形可以是带孔洞的复合路径。非圆形径向渐变、偏离中心的径向渐变焦点也会返回错误。

坐标输出最多保留 6 位小数，格式化后变成零的尺寸会被拒绝。组透明度和非均匀描边变换沿用官方近似，具体边界见[转换基线](docs/upstream.md)。SVG 与 VectorDrawable 的表达能力不同，转换完成后仍应在目标 Android 版本检查效果。

## Rust 库

```rust
use svg2vector::{Options, convert};

fn main() -> Result<(), svg2vector::Error> {
    let svg = r##"<svg viewBox="0 0 24 24"><path fill="#e8495c" d="M2 2H22V22H2Z"/></svg>"##;
    let xml = convert(svg, &Options::default())?;
    println!("{xml}");
    Ok(())
}
```

## 验证

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked
```

测试覆盖几何变换、样式、引用、裁剪、渐变和实际 CLI 的输入输出及错误路径。外部设计素材仅用于本地转换对照，不随项目分发。

## 许可证

Apache-2.0。AOSP 算法来源见 [NOTICE](NOTICE)。
