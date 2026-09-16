# 转换基线

本项目以 AOSP 的 `Svg2Vector` 转换流程为基线，将 SVG 节点、样式、引用、路径变换和渐变坐标处理迁移为 Rust。XML 解析使用 `roxmltree`，SVG 词法和路径解析使用 `svgtypes`；转换时不调用 JVM 或 Android SDK。

## 固定版本

- Maven 坐标：`com.android.tools:sdk-common:31.13.1`
- [官方源码包](https://dl.google.com/dl/android/maven2/com/android/tools/sdk-common/31.13.1/sdk-common-31.13.1-sources.jar)
- 源码包 SHA-256：`d628ff3faef817facebd17b6befbc32f13ec8719fc6502ac29c7f8820fcfdff9`
- [官方源码浏览入口](https://cs.android.com/android-studio/platform/tools/base/+/mirror-goog-studio-main:sdk-common/src/main/java/com/android/ide/common/vectordrawable/Svg2Vector.java)

源码浏览入口跟随分支更新；当前行为基线以固定 Maven 版本为准。

## 模块对应

| AOSP 模块 | Rust 模块 |
| --- | --- |
| `Svg2Vector`、`SvgTree` | `parser.rs`、`dimensions.rs` |
| `SvgNode`、`SvgGroupNode`、`SvgLeafNode` | `styles.rs`、`parser_css.rs`、`shapes.rs` |
| `PathParser`、`VdPath.Node`、`EllipseSolver` | `geometry.rs`、`geometry_path.rs`、`svgtypes` |
| `SvgGradientNode`、`GradientStop`、`SvgColor` | `paint*.rs` |
| `SvgClipPathNode` | `parser_clip.rs` |
| 各节点的 XML 输出 | `writer.rs` |

## 等价性与差异

这不是完整 AOSP 工具包的逐行翻译。当前版本覆盖常见图标特性，并对未覆盖的视觉特性返回错误。

- 椭圆弧由 `svgtypes` 规范化为三次贝塞尔曲线，因此路径字符串可能与 Java 版本不同；验收比较渲染结果。
- 坐标保留最多 6 位小数。
- `rgba()` 的 alpha 按 SVG/CSS 的 `0..1` 或百分比解释。
- 重复图层 ID 按官方引用表规则由后出现的定义覆盖。
- 渐变使用局部边界与完整变换矩阵，修正官方算法在旋转、非均匀缩放和偏移边界上的插值误差；用户坐标渐变的省略值按 SVG 百分比默认值计算。
- 多图形裁剪需要几何并集运算，当前明确拒绝，避免将轮廓拼接后错误地消除重叠区域。单个复合路径的孔洞保持原有填充规则。
- 非均匀变换下的描边宽度沿用官方的行列式平方根近似。
- 曲线边界沿用官方控制点包围盒，未计算贝塞尔曲线的精确极值；边界相关的渐变、裁剪可能与浏览器存在差异。
- 组透明度沿用转换时向子路径累乘的方式；重叠子图形的合成效果可能与浏览器不同。
- 预览渲染器 `VdPreview` 不属于 CLI 的运行依赖。开发阶段可以使用官方工具生成对照结果。

移植的算法保留 AOSP 来源与 Apache-2.0 声明，见 `NOTICE` 和 `LICENSE`。
