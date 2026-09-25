# Sena Spine Runtime 对接方案 v1

## 1. 结论

Sena 正式 Runtime：

```text
Rust behavior/context
        ↓
Spine Director
        ↓
official spine-c 3.8 runtime
        ↓
Sena render extraction
        ↓
Direct3D 11
        ↓
DXGI composition swap chain
        ↓
DirectComposition
        ↓
transparent topmost Win32 pet window
```

设置窗口继续使用 Slint。

Sprite renderer 保留为 fallback。

## 2. 为什么使用 spine-c

不采用第三方 Rust Spine Runtime 作为正式依赖。

使用官方 generic runtime：

- spine-c。
- 与 Spine Editor / 官方 Runtime 同版本线。
- Rust 通过 C ABI 调用。
- AnimationState、Skin、Mesh、Clipping 等 Spine 3.8 动画逻辑由官方 Runtime 负责。
- Sena 实现 Windows 渲染适配、安全 Rust wrapper，以及独立的轻量 secondary-motion spring system。

不要自己解析 Spine JSON/binary 实现动画系统。

## 3. License Gate

在真正集成 Spine Runtime 之前必须满足：

1. 项目持有有效的 Spine Editor license。
2. Trial 只用于评估，不作为正式 Runtime 集成依据。
3. 分发物中保留 Spine Runtimes license/copyright notice。
4. Sena 自己仍可保持 Apache-2.0，但 Spine Runtime 代码和许可声明必须明确作为第三方组件处理。
5. `THIRD_PARTY_NOTICES.md` 必须列出 Spine Runtime。

生产规范锁定 Spine 3.8 Professional，因为正式方案需要 weighted meshes、deform、skins 与完整约束能力。运行时二级运动不依赖 4.x Physics Constraints。

## 4. Version Lock

第一版锁定：

```text
Spine Editor      : 3.8.75 Professional
Spine Runtime line: 3.8
```

如果以后升级 Editor，必须把 Editor 与 Runtime 作为一次完整迁移处理；当前阶段不为了追新版本打断资产制作。

仓库记录：

```text
third_party/spine-runtimes -> exact commit (initialize with `git submodule update --init --recursive`)
pets/sena/spine/project/*.spine
pets/sena/spine/settings/export.json
pets/sena/spine/export/*
```

规则：

- Runtime major.minor 变化必须重新导出 skeleton data。
- Patch 更新也必须跑完整视觉回归。
- binary export 不视为源文件，`.spine` 才是源文件。

## 5. Export Format

### Dev

允许：

```text
sena.json
sena.atlas
sena.png
```

用途：

- diff / inspect。
- 开发工具。
- 验证 attachment / animation 名。

### Release

使用：

```text
sena.skel
sena.atlas
sena.png
```

理由：

- binary 体积更小。
- 加载更快。

Cat 同样独立导出。

## 6. Rust FFI 边界

建议目录：

```text
native/
  spine_bridge/
    CMakeLists.txt
    sena_spine_bridge.c
    sena_spine_bridge.h

src/render/spine/
  mod.rs
  ffi.rs
  runtime.rs
  skeleton.rs
  animation.rs
  skin.rs
  renderer.rs
  d3d11.rs
  window.rs
  hit_test.rs
  error.rs
```

不要让大量 `unsafe extern "C"` 泄漏到业务代码。

Rust 层只暴露安全对象：

```text
SpineRuntime
SpineSkeleton
SpineAnimationState
SpineSkinSet
SpineRenderFrame
SpineEvent
```

生命周期：

- Atlas / SkeletonData 由 Runtime owner 持有。
- Skeleton 不得比 SkeletonData 活得更久。
- Texture GPU 资源独立由 renderer 管理。
- C 指针永远封装在私有类型中。

## 7. RendererKind / Package Schema

宠物包 schema 已支持：

```text
renderer: "spine"
```

以及：

```json
{
  "spine": {
    "skeleton": "spine/export/sena.skel",
    "atlas": "spine/export/sena.atlas",
    "scale": 1.0,
    "default_skin": "base"
  }
}
```

当前正式 `pet.json` 仍保持 Sprite，以保证应用一直可运行；`pet.template.json` 已携带 Spine 迁移配置。等 R3 有可加载的 Sena skeleton/atlas 后，再切默认 renderer。Sprite schema 和资源继续作为 fallback。

## 8. Render Extraction

每帧：

1. `AnimationState.update(dt)`。
2. `AnimationState.apply(skeleton)`。
3. 更新 skeleton world transforms。
4. 运行 Sena secondary-motion spring pass，并再次刷新受影响骨骼/slot world transforms。
5. 按 slot draw order 遍历。
6. Region attachment -> 4 顶点 / 6 indices。
7. Mesh attachment -> world vertices + UV + triangles。
8. Clipping attachment -> 官方 runtime clipping。
9. 按 texture + blend mode 合批。
10. 提交 D3D11 dynamic vertex/index buffers。
11. Present composition swap chain。

GPU Vertex：

```text
position : float2
uv       : float2
light    : rgba8 or float4
dark     : rgba8 or float4
```

预留 two-color tint，即使第一批美术暂时不用。

## 9. Blend / Alpha

生产统一：

- Atlas：premultiplied alpha。
- Composition swap chain：premultiplied alpha。
- Window 背景清零为透明黑。
- Shader / blend state 按 Spine slot blend mode 选择。

必须支持：

- Normal
- Additive
- Multiply
- Screen

首个 smoke test 至少先验证 Normal + Additive。

## 10. Windows 透明窗口

正式宠物窗口不继续依赖普通 HWND wgpu surface 的 alpha 能力。

使用：

- D3D11 Device。
- `CreateSwapChainForComposition`。
- flip-model swap chain。
- `DXGI_ALPHA_MODE_PREMULTIPLIED`。
- DirectComposition Visual。
- borderless / topmost Win32 host window。

这样透明能力由 Windows Composition 路径明确提供，而不是依赖某个 wgpu backend 恰好暴露 alpha mode。

## 11. Window 与坐标

Spine skeleton world：

- root = 脚底中心。
- +X 右。
- +Y 上。

Windows：

- top-left origin。

Renderer 做一次坐标变换：

```text
screen_x = anchor_x + spine_x * scale
screen_y = baseline_y - spine_y * scale
```

桌面移动逻辑只移动 window/root，不修改 Spine animation root motion。

动画中禁止用 root translation 让角色“真的走过桌面距离”；walk 只表现步态，真实位移由现有 pet motion controller 管理。

## 12. Surface 尺寸

不要每帧按 skeleton AABB 重建 swap chain。

第一版使用稳定透明画布：

- 默认逻辑区域约 640 × 640。
- 角色正常高度约 320–420 px。
- 坐 / 睡等宽姿态预留透明边距。
- Hit Test 只使用实际可见 attachment，不用整个窗口矩形。

如果未来确实需要更宽的 sleeping pose，再升级成“状态级固定 surface profile”，而不是逐帧 resize。

## 13. Hit Testing

Sprite 当前的 alpha hit test 不能直接照搬。

Spine hit test 分两级：

### Level 1 — Geometry

- 当前可见 attachment world triangles。
- point-in-triangle。
- clipping 后的 triangle 才算。

### Level 2 — Texture Alpha

只有 Level 1 命中后：

- 由 barycentric UV 得到 atlas texel。
- 读取 CPU-side alpha mask。
- alpha >= threshold 才命中。

缓存每张 atlas 的 alpha mask。

这样头发透明边缘不会变成大矩形点击区。

## 14. Animation Director

新增一个业务层，不让 Behavior 直接操作 Spine track：

```text
DesktopContext / Behavior
        ↓
PetPresentationState
        ↓
SpineDirector
        ↓
AnimationState tracks / skins / events
```

`PetPresentationState` 示例：

```text
posture: Standing | Sitting | Sleeping | CarryingCat
activity: Idle | Coding | Listening
facing: Left | Right
typing: bool
headphones: bool
laptop: bool
expression: Neutral | Happy | Curious | Sleepy | Focused
```

Director 负责决定：

- transition。
- track。
- skin composition。
- props。
- mix duration。
- interrupt policy。

## 15. Event Pipeline

Spine event -> Rust enum：

```text
FacingLeft
FacingRight
FootLeft
FootRight
HeadphonesOn
HeadphonesOff
LaptopOn
LaptopOff
CatAttach
CatDetach
SleepCommitted
WakeCommitted
```

未知事件：

- Debug：日志 warning。
- Release：忽略但计数，不 panic。

## 16. Cat Runtime

Cat 作为独立 `SpineSkeleton`。

普通状态：

- 自己的 AnimationState。
- 自己的 root world position。

Carry：

- Sena animation 发出 `cat_attach`。
- Runtime 每帧读取 Sena `cat_carry` bone world transform。
- 把 Cat root 对齐到该 transform。
- Cat 播放 `cat_carry_idle`。
- 放下时 `cat_detach` 后做短 world-space interpolation。

Sena / Cat blink RNG 使用不同 seed + 不同 phase。

## 17. Secondary Motion Spring

Spine 3.8 的正式方案不依赖 Physics Constraints。Sena 在 Rust 层实现轻量 spring solver，只对明确登记的骨骼链叠加增量旋转/位移。

每条 spring chain 配置：

```text
root_bone
tip_bones[]
stiffness
damping
gravity
inertia
max_angle
max_offset
```

更新顺序：

```text
Spine AnimationState
 -> skeleton world transform
 -> sample pet/window acceleration
 -> spring integration
 -> apply additive bone offsets
 -> refresh affected world transforms
 -> render
```

桌面移动规则：

- 正常走动：window/root 加速度进入 spring input，形成头发、裙摆自然滞后。
- 拖拽：允许更明显 inertia，但必须 clamp 最大角度/偏移。
- 急停：spring 自然衰减，不瞬间归零。
- 显示器切换、窗口安全纠正等瞬移：调用 hard reset，避免发丝爆飞。
- 切换到跨度很大的 posture（例如 sleep）：做 soft reset/短 blend，避免上一姿态的速度残留。
- Sleeping：spring 可以降频或冻结到 authored fallback。
- 关闭 secondary motion 时，角色仍靠 Spine 关键帧保持基本生动。

第一版只实现旋转型链条；确实需要后再增加 translation spring。

## 18. Update / Render Rate

建议：

### Active locomotion / interaction

- render: 60 FPS。
- Spine update: 60 Hz。

### Ordinary idle

- render/update: 30 Hz。
- 没有 spring / micro motion 时允许更低。

### Sleeping

- 10–20 Hz 足够。
- 没有可见变化时可暂停 present，等下一个定时事件。

### Hidden / locked

- 停止 render。
- 停止 spring update。
- 仅保留必要的 context timer。

## 19. Texture / Batch Budget

单 Sena：

- 1 atlas page 优先，最多 2。
- 2048² preferred。
- 运行时 vertices 目标 < 1500。
- draw calls 目标：
  - Normal-only frame：尽量 < 10。
  - 多 blend/多 page：< 20。

只有一个桌宠角色，不需要为几百个 skeleton 的极端场景过度优化。

## 20. Error / Fallback

任意情况：

- Spine DLL/static runtime 初始化失败。
- skeleton 版本不匹配。
- atlas 缺失。
- texture 解码失败。
- animation contract 缺失关键项。
- D3D11/DirectComposition 初始化失败。

都不得导致应用无法启动。

顺序：

```text
Spine renderer
  -> Sprite official package
  -> Placeholder
```

Settings / tray 始终可打开。

## 21. 里程碑

### R0 — License / Version Gate

- 确认 Spine license。
- 锁定官方 spine-c 3.8 runtime commit：`c0699e23a0c8799710323bdf0e076e18f6ba41a2`（最后一个兼容 Spine Editor 3.8.75 导出格式的提交；后续 3.8 分支提交已明确拒绝 3.8.75 skeleton）。
- third-party notice。

### R1 — Skeleton Runtime Core ✅

已完成：

- 官方 spine-c 已作为 pinned Git submodule 接入。
- 由于 Editor 固定为 3.8.75 Professional，Runtime 精确锁定在 `c0699e23a0c8799710323bdf0e076e18f6ba41a2`。
- C bridge 隔离官方裸指针与 3.8 legacy API 边界。
- Rust safe wrapper 可创建 skeleton、从文件加载 JSON + Atlas、设置 track animation、update/apply 并读取 bone world transform。
- 自动 smoke fixture 以 `spine: 3.8.75` 播放 `idle`，验证 root translation / rotation。
- 文件加载 smoke test 已验证 `sena.json + sena.atlas`；`.skel` binary loader 已接入 bridge，等第一份真实 Spine 3.8.75 导出后做真实资产回归。
- 缺失动画名在 bridge 中预检查，避免 3.8 `setAnimationByName` 的 null animation 崩溃。
- spine-c 调用在 Rust 层串行化，避免未来跨线程直接触碰 C runtime 状态。
- R1 不加载 atlas texture；纹理生命周期从 R2 D3D11 renderer 开始。

无透明窗口要求，下一阶段进入 R2。

### R2 — Renderer Core / Transparent Window

#### R2A — Render extraction + ordinary D3D11 preview ✅

已完成：

- 按 Spine slot draw order 提取 Region / Mesh / LinkedMesh。
- 使用官方 `spSkeletonClipping` 处理 clipping attachments。
- 提取 UV、triangle indices、light tint、dark tint、atlas page、slot / attachment name。
- 映射 Normal / Additive / Multiply / Screen 四种 blend mode。
- 使用官方 Spineboy 3.8 Pro 真实资产回归，覆盖 weighted mesh 与 clipping。
- `examples/spine_d3d11_preview.rs` 创建原生 D3D11 swap chain，加载 PMA atlas PNG，并真正播放 `idle`。
- D3D11 preview 已实现四种 Spine PMA blend state。
- Rasterizer 明确使用 `D3D11_CULL_NONE`。Spine 三角形经过 Y 轴翻转后 winding 会改变，不能依赖 D3D11 默认 back-face culling。
- 自动 `--frames 5` smoke test 已在 Windows 上实际创建窗口、绘制并 Present。

开发预览：

```powershell
cargo run --example spine_d3d11_preview -- --animation idle
cargo run --example spine_d3d11_preview -- --animation walk --frames 300
```

当前 preview 为验证渲染链路的普通 HWND；它允许每个 attachment/frame 创建临时 immutable buffers。正式桌宠 renderer 会改为可复用 dynamic/ring buffers，避免把 smoke 实现直接带进生产。

#### R2B — DirectComposition transparent pet window ✅

已完成：

- 新增 production-facing `src/render/spine/dcomp.rs`，不再把透明链路只放在 example 中。
- D3D11 device 使用 `D3D11_CREATE_DEVICE_BGRA_SUPPORT`。
- 通过 `IDXGIFactory2::CreateSwapChainForComposition` 创建 windowless composition swap chain。
- swap chain 使用 `DXGI_FORMAT_B8G8R8A8_UNORM`、双缓冲、`DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL`。
- alpha mode 固定为 `DXGI_ALPHA_MODE_PREMULTIPLIED`。
- `IDCompositionVisual::SetContent` -> `IDCompositionTarget::SetRoot` -> `Commit` 已跑通。
- 每帧 render target 使用 `[0, 0, 0, 0]` 清屏，角色像素走 PMA blend。
- Normal / Additive / Multiply / Screen 四种 Spine PMA blend 均保留。
- 新增 `examples/spine_dcomp_preview.rs`。
- preview HWND 使用 `WS_POPUP | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_NOREDIRECTIONBITMAP`，不使用 `WS_EX_LAYERED`。
- Windows 实机已完成 `--frames 5` Present smoke test。
- Rasterizer 同样固定 `D3D11_CULL_NONE`。早期只验证 Present/透明清屏时曾掩盖三角形被默认 back-face culling 丢弃的问题；现已通过真实角色像素回归覆盖。
- DirectComposition example 在角色真实可见后做同一区域窗口前/后像素对比，约 26% 区域出现角色变化，其余区域继续透出桌面。

开发预览：

```powershell
cargo run --example spine_dcomp_preview -- --animation idle
cargo run --example spine_dcomp_preview -- --animation walk --frames 300
```

R2B 当前仍保留 per-batch immutable vertex/index buffer，目的是先锁定透明 composition 路径。正式持续运行前应改为可复用 dynamic/ring buffers。

下一阶段进入 R3：用 Sena 自己的 Spine atlas/setup pose/idle 替换验证用 Spineboy，并把 production renderer 接入桌宠 presentation lifecycle。

### R3 — Sena Still / Idle

#### R3A — Main presentation lifecycle ✅

已完成：

- `pet.json` Spine schema 增加 Behavior -> Spine animation 的语义映射和 interaction animation 名称合同。
- Runtime 支持应用 `default_skin`，切 skin 后恢复 slot setup pose。
- 主程序检测 `renderer: "spine"` 后，不再依赖 Sprite frame timer；由 16 ms presentation tick 驱动 Spine Runtime / D3D11。
- Behavior 可直接切换 track 0 动画；请求动画不存在时回退 `idle`。
- 用户缩放变化会销毁并重建 Spine presentation，使 composition surface 与桌宠尺寸同步。
- Slint `PetWindow` 新增 `use-spine` 模式：角色视觉隐藏，只保留透明 TouchArea、拖拽、点击和右键等现有交互路径。
- 不把 DirectComposition visual 直接挂到 Slint 自己的 HWND。正式方案使用独立 `SenaSpineCompositionHost` HWND，避免 Slint renderer 与 DComp 争用同一窗口 surface。
- composition host 使用 no-activate / no-redirection popup，跟随 Slint 交互窗口的位置、尺寸和显隐；`WM_NCHITTEST -> HTTRANSPARENT`，鼠标继续交给 Slint 交互窗。
- Spine/D3D11 presentation 初始化或运行期失败会销毁 presentation，并恢复 Slint placeholder，避免留下不可见交互窗口。
- 用临时官方 Spineboy 3.8 package 直接启动正式 `sena.exe` 做主程序集成烟测：桌宠交互窗口约 296×420，运行前后像素对比约 29.65% 为真实角色变化、约 69.5% 保持桌面原像素，确认“主程序行为系统 + Slint input overlay + 独立 DComp 角色窗口”完整链路成立。
- 正式 `pets/sena/pet.json` **仍保持 Sprite**，在 Sena 自己的 Spine 导出资产到位前不切默认 renderer。

#### R3B — Sena asset gate（基础设施 ✅ / 等待真实资产）

已完成自动验收基础设施：

- 新增 `examples/sena_spine_asset_gate.rs`。
- spine-c bridge 可读取 skeleton runtime version。
- 可枚举 skin、animation 名称和 animation duration。
- 可枚举 atlas texture pages。
- gate 会验证所有 atlas page 文件真实存在。
- gate 会验证 setup pose 可提取出有效 batches / vertices / triangle indices / bounds。
- gate 会应用 `base` skin，并验证 `root/body_root/head/face_root/eye_l/eye_r`。
- 当前 R3B 必需动画：`idle`、`blink_l`、`blink_r`。
- 会实际播放 `idle` 并再次提取 render frame。
- 顶点数超过 1500 给 warning，超过约 2500 给更强 warning。
- 会列出后续 R4/R5 仍缺少的 animation 名称，但不阻塞第一份静态/idle 导入。
- 已用官方 Spineboy 3.8.55 Pro export 实测 inventory：版本、skin、11 个 animations、animation durations 和 atlas page 均可正确读取。

默认验收命令：

```powershell
cargo run --example sena_spine_asset_gate
```

默认查找：

```text
pets/sena/spine/export/sena.skel
pets/sena/spine/export/sena.atlas
```

若 binary skeleton 尚未导出，会尝试开发期 `sena.json`。

现在真正剩下的 R3B 输入只有第一份 Sena 3.8.75 Professional 导出：

- `sena.skel` 或开发期 `sena.json`。
- `sena.atlas`。
- atlas PNG。
- `base` skin。
- `idle`。
- `blink_l` / `blink_r`。
- setup pose。

拿到真实资产后：

1. 运行 asset gate。
2. 用真实 Sena weighted mesh / clipping / skin 做 runtime 回归。
3. 做静态 setup pose 视觉 Gate。
4. 把当前整窗 TouchArea 收窄为 attachment geometry + alpha hit testing。
5. 全部通过后才把正式 `pet.json` 切到 `renderer: "spine"`。

### R4 — Locomotion

- walk_l / walk_r。
- turn。
- desktop motion controller 驱动位置。
- Rust secondary-motion spring。

### R5 — Context State

- coding。
- listening。
- drowsy。
- sleep。
- interaction tracks。

### R6 — Cat

- separate skeleton。
- pickup / carry / putdown。
- independent blink。

### R7 — Production

- power usage。
- DPI。
- multi-monitor。
- crash fallback。
- release packaging。
- license notices。

## 22. 测试

### Unit

- manifest path validation。
- animation contract lookup。
- event mapping。
- state transition planning。
- mix duration。
- fallback selection。

### Native smoke

- create D3D11 device。
- create composition swap chain。
- render 120 frames。
- alpha surface verification。
- resize/DPI。
- device-lost handling。

### Visual regression

固定输出：

- idle。
- walk L/R。
- turn L->R / R->L。
- sit。
- coding。
- listening。
- sleep。
- cat carry。

每个录制短视频或关键帧图用于人工审核。
