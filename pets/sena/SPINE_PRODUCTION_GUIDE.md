# Sena Spine 2D 生产规范 v1

> 状态：**Production Direction / Active**
>
> 从 2026-09 起，Sena 桌宠的正式美术路线由 3D/VRM 切换为 **Spine 2D 骨骼动画**。
> 旧 Sprite 继续作为安全 fallback；V1/V2/V3 3D 资产仅保留为历史实验与动作参考，不再作为正式外观来源。

## 1. 目标

Sena 必须首先满足“静止就好看”，其次才是“动起来顺”。

正式目标：

- 日系 Q 版年轻女性角色，约 **2.8–3.2 头身**，不能幼儿化。
- 银白/月光色超长发，带淡粉紫冷光。
- 紫粉色大眼，但避免圆球玩具感。
- 大型淡紫半透明水晶蝴蝶结是第一轮廓识别点。
- 白 / 淡紫 / 冰蓝分层裙装。
- 与奶油橘白猫形成统一画风。
- 桌面默认显示高度约 320–420 px 时仍能看清脸、蝴蝶结、猫和主要动作。
- 正面、3/4、侧向动作不能依赖“糊成一团”来掩盖结构。

## 2. 工具线

正式制作锁定：

- Spine Editor：**3.8.75 Professional**（当前制作机版本）。
- Runtime：官方 **spine-c 3.8** 系列 + Sena 自己的 Windows renderer。
- PSD / 分层 PNG：绘画工具不限，但必须遵守本文拆层合同。
- 二级运动：Spine 内制作基础跟随姿态，运行时由 Sena 的 Rust spring system 增强；不依赖 4.x Physics Constraints。

版本规则：

1. Spine Editor 与 Spine Runtime 必须保持同一 **major.minor**。
2. `.spine` 源工程必须长期保存。
3. 每次导出都记录 Editor 版本与 Runtime commit。
4. 不允许“更新 Runtime 后继续使用旧 binary export 而不复验”。
5. Release 用 binary skeleton；开发期可额外导出 JSON 便于检查。

## 3. 源图尺寸与色彩

### 3.1 工作分辨率

第一版建议：

- 角色站立总高度：**2048–3072 px**。
- sRGB。
- 透明背景。
- 72/300 DPI 不作为运行时约束，像素尺寸才是约束。
- 不把环境光、地面阴影、外发光烘焙进角色主图。

桌宠最终只显示几百像素高，因此不需要 8K 角色原画。高分辨率源图用于网格变形和抗锯齿，不用于直接运行。

### 3.2 关节隐藏余量

所有会相对运动的部件必须有被遮住的延伸区域：

- 上臂进入袖子：至少保留约 15–25% 隐藏长度。
- 前臂进入上臂：至少 10–20%。
- 大腿进入裙摆：至少 20%。
- 小腿进入鞋 / 袜：至少 10–15%。
- 刘海、侧发根部必须进入头顶覆盖区。
- 裙摆分片之间必须有重叠，不能只靠边缘刚好拼接。

目的：骨骼旋转后不能露透明缝。

## 4. PSD 拆层合同

命名统一使用 `snake_case`，禁止中文层名进入生产 PSD。

### 4.1 头部

```text
head/
  head_base
  ear_l
  ear_r

  face/
    brow_l
    brow_r
    eye_white_l
    eye_white_r
    iris_l
    iris_r
    pupil_l
    pupil_r
    eye_highlight_l
    eye_highlight_r
    eyelid_upper_l
    eyelid_upper_r
    eyelid_lower_l
    eyelid_lower_r
    lash_l
    lash_r
    blush_l
    blush_r
    mouth_neutral
    mouth_smile
    mouth_open
    mouth_sleep
```

眼睛必须左右独立，Sena 左右眨眼不得强制同步。

### 4.2 头发

```text
hair/
  back/
    hair_back_c
    hair_back_l1
    hair_back_l2
    hair_back_r1
    hair_back_r2

  side/
    hair_side_l1
    hair_side_l2
    hair_side_r1
    hair_side_r2

  front/
    bang_l2
    bang_l1
    bang_c
    bang_r1
    bang_r2
    ahoge

  highlight/
    hair_highlight_l
    hair_highlight_r
```

长发禁止画成一整块不可变形的披风。大块后发至少拆成 5 个可独立摆动的主要质量块。

### 4.3 蝴蝶结

```text
bow/
  bow_knot
  bow_upper_l
  bow_upper_r
  bow_lower_l
  bow_lower_r
  bow_tail_l
  bow_tail_r
  bow_crystal
  bow_glow
```

`bow_glow` 可以单独做 Additive，其他主体保持 Normal/PMA。

### 4.4 身体

```text
body/
  torso
  hips

  arm_l/
    upper_arm_l
    forearm_l
    hand_l

  arm_r/
    upper_arm_r
    forearm_r
    hand_r

  leg_l/
    thigh_l
    calf_l
    foot_l

  leg_r/
    thigh_r
    calf_r
    foot_r
```

### 4.5 服装

```text
dress/
  bodice
  collar
  sleeve_l
  sleeve_r

  skirt/
    skirt_back
    skirt_mid_l
    skirt_mid_r
    skirt_front_l
    skirt_front_c
    skirt_front_r
    chiffon_l
    chiffon_r
    crystal_hem

  shoes/
    shoe_l
    shoe_r

  decor/
    waist_crystal
    butterfly_l
    butterfly_r
    star_01
    star_02
```

裙摆必须分前、中、后层。透明雪纺和水晶边缘不要与主体裙布合成一张图。

### 4.6 道具

```text
props/
  headphones
  laptop
  pillow
  blanket
```

耳机与电脑不直接画死在角色身上，必须是独立 attachment。

## 5. 方向素材

正式桌宠不把“整个角色 scaleX = -1”作为最终左右转向方案。

原因：

- 蝴蝶结、发型与服装存在明显非对称设计。
- 镜像会改变角色标志物所在身体侧。
- 左右转身时遮挡关系不同。

生产方案：

- `dir_r`：向右素材 Skin。
- `dir_l`：向左素材 Skin。
- 对真正对称的附件允许复用。
- 对头发、蝴蝶结、裙摆、脸侧轮廓、手持道具制作左右方向 attachment。
- `turn_l_to_r` / `turn_r_to_l` 是真实动画，不是瞬间镜像。

## 6. Spine Skeleton 层级

建议基础骨架：

```text
root
└─ body_root
   ├─ hips
   │  ├─ torso
   │  │  ├─ chest
   │  │  │  ├─ neck
   │  │  │  │  └─ head
   │  │  │  │     ├─ face_root
   │  │  │  │     │  ├─ eye_l
   │  │  │  │     │  ├─ eye_r
   │  │  │  │     │  ├─ brow_l
   │  │  │  │     │  └─ brow_r
   │  │  │  │     ├─ hair_front_root
   │  │  │  │     ├─ hair_side_l_root
   │  │  │  │     ├─ hair_side_r_root
   │  │  │  │     ├─ hair_back_root
   │  │  │  │     └─ bow_root
   │  │  │  ├─ shoulder_l
   │  │  │  │  └─ upper_arm_l
   │  │  │  │     └─ forearm_l
   │  │  │  │        └─ hand_l
   │  │  │  └─ shoulder_r
   │  │  │     └─ upper_arm_r
   │  │  │        └─ forearm_r
   │  │  │           └─ hand_r
   │  ├─ thigh_l
   │  │  └─ calf_l
   │  │     └─ foot_l
   │  └─ thigh_r
   │     └─ calf_r
   │        └─ foot_r
   ├─ skirt_root
   ├─ cat_carry
   ├─ right_hand_prop
   ├─ left_hand_prop
   ├─ headphones_anchor
   └─ laptop_anchor
```

## 7. Mesh / Weight 规则

### 7.1 什么时候用 Region

以下部件默认保持普通 region attachment：

- 眼睛高光
- 眉毛
- 瞳孔
- 小装饰
- 水晶小挂件
- 大部分道具
- 不需要弯曲的鞋子

### 7.2 什么时候用 Mesh

优先给这些部件做 weighted mesh：

- 脸颊 / 头部轮廓（只做轻微形变）
- 刘海与长发
- 袖子
- 裙摆
- 雪纺
- 大蝴蝶结翼与尾带
- 猫尾巴和身体软组织

### 7.3 顶点预算

单只 Sena 的生产目标：

- 可变形 Mesh：约 20–35 个。
- 运行时总有效顶点：目标 **< 1500**，上限约 2500。
- 单顶点骨骼影响：优先 1–2 个，上限 4 个。
- 不为“看起来更专业”无意义加密网格。

原则：形变主要靠骨骼权重，deform timeline 只做必要的小修正。

## 8. 二级运动规则（Spine 3.8）

Spine 3.8 不把 4.x Physics Constraints 作为生产依赖。二级运动拆成两层：

1. **Spine authored fallback**：动画师给长发、裙摆、蝴蝶结、猫尾巴做轻微跟随关键帧，保证即使关闭运行时 spring 也不僵硬。
2. **Rust runtime spring**：运行时在基础动画结果上叠加轻量弹簧骨骼偏移，负责移动、拖拽和急停时的惯性。

适合做 spring chain：

- 后发 3–5 组。
- 侧发 2–4 组。
- 蝴蝶结尾带。
- 裙摆外层。
- 雪纺。
- 猫尾巴。

不做 spring：

- 眼睛。
- 主躯干。
- 手脚主运动。
- 坐下 / 抱猫等关键姿态骨骼。

第一版目标约 **8–16 条 secondary-motion bone chains**。每条链都必须有合理的 authored rest/fallback 动画，spring 只做增量，不接管主姿态。

## 9. Skin 设计

正式 Skin 分成可组合层：

```text
base
dir_r
dir_l
outfit_default
prop_headphones
prop_laptop
expression_fx
```

Runtime 组合顺序：

```text
base -> direction -> outfit -> props
```

同一 slot 的后层 Skin 可以覆盖前层附件。

猫不塞进 Sena Skin。猫是独立 Spine skeleton。

## 10. 猫资产

猫单独拥有：

```text
sena_cat.spine
```

猫的 root 由 Runtime 控制：

- 地面状态：独立世界坐标。
- 被抱起：猫 root 跟随 Sena 的 `cat_carry` bone。
- 放下：从 `cat_carry` 世界坐标平滑过渡到地面坐标。

猫眨眼与 Sena 必须使用不同随机相位，禁止同帧同步眨眼。

## 11. Atlas / Texture

第一版：

- 主 Atlas：2048 × 2048。
- 尽量 1 page；必要时最多 2 pages。
- Texture：PNG, sRGB。
- 运行时统一 premultiplied alpha。
- 至少保留 2–4 px packing padding。
- 不允许明显透明白边 / 紫边。

桌宠默认显示很小，因此 Atlas 的目标是“缩小时仍干净”，而不是堆超高分辨率。

## 12. Blend Mode

Runtime 必须最终支持 Spine 常用 slot blend：

- Normal
- Additive
- Multiply
- Screen

但首批正式素材尽量使用：

- Normal：绝大多数角色。
- Additive：水晶光、星光、小特效。

不要用 Multiply/Screen 去补救本该在原画中解决的问题。

## 13. 文件布局

```text
pets/sena/spine/
  source/
    sena.psd
    cat.psd

  project/
    sena.spine
    sena_cat.spine

  export/
    sena.skel
    sena.atlas
    sena.png
    sena_cat.skel
    sena_cat.atlas
    sena_cat.png

  settings/
    export.json
    texture_packer.json
```

PSD、Spine 工程和 binary export 使用 Git LFS。

## 14. 美术验收 Gate

### Gate A — Still Pose

只看静止 setup pose：

- 正面可爱。
- 3/4 可爱。
- 左右侧向都能认出星奈。
- 头发不是一团。
- 蝴蝶结轮廓清楚。
- 手脚比例自然。

Gate A 不通过，禁止做动画。

### Gate B — Deformation

测试：

- 头左右转。
- 手举起。
- 坐下。
- 单腿抬起。
- 裙摆摆动。
- 长发大幅摆动。

不能出现：

- 关节透明缝。
- 网格尖刺。
- 贴图拉伸到明显糊掉。
- 脸部轮廓塌陷。

### Gate C — Runtime Scale

在约 320–420 px 桌面高度下检查：

- 眼睛仍清晰。
- 蝴蝶结仍能认出。
- 长发层次仍存在。
- 小特效不过曝。
- 轮廓抗锯齿干净。

## 15. 禁止事项

- 不再从 Python 从零生成正式人物 Mesh。
- 不用整张立绘做一个四边形然后骨骼硬拉。
- 不把所有长发做成一个 mesh。
- 不让运行时 spring 代替关键动画。
- 不靠 scaleX 镜像解决正式左右朝向。
- 不在动作完成前把 transition 当成“以后再补”。
