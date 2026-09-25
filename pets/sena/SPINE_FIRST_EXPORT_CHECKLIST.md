# Sena Spine 3.8 第一份可运行导出清单

> 目标：只完成第一份可被 Sena Runtime 正确加载的 **Sena 本人**。
>
> 这一阶段不追求功能数量，只追求静止质量、基础骨架正确、Idle 自然、左右眼能够独立眨眼。

## 0. 开始前

工具固定：

- Spine Editor **3.8.75 Professional**。
- 项目源文件：`pets/sena/spine/project/sena.spine`。
- 绘画源文件：`pets/sena/spine/source/sena.psd`。
- 第一份导出目录：`pets/sena/spine/export/`。
- 命名全部使用小写 `snake_case`。

先运行合同检查：

```powershell
cargo run --example sena_spine_asset_gate -- --contract-only
```

必须看到：

```text
[PASS] R3B contract is valid.
```

## 1. 美术 Gate A

第一份正式原画只包含 Sena 本人。

不要放入：

- 猫。
- 耳机。
- Laptop。
- 枕头。
- 毯子。
- Zzz。
- 音符。
- 地面阴影。
- 其他 UI / 粒子。

母稿必须先满足 `SPINE_SETUP_POSE_BRIEF.md`。

关键视觉要求：

- 约 2.8–3.2 头身。
- 年轻女性感，不幼儿化。
- 银白/月光长发。
- 紫粉眼睛。
- 大型淡紫半透明水晶蝴蝶结。
- 白 / 淡紫 / 冰蓝分层短裙。
- 轻薄鞋底。
- 双臂、双腿与身体留有可拆分空间。
- 缩到 320–420 px 高仍然漂亮。

如果 flatten preview 本身不好看，不进入 Spine。

## 2. PSD 拆层

使用：

```text
pets/sena/spine/settings/layer_contract.json
```

作为拆层命名来源。

第一份工程至少保证以下区域是独立可编辑素材：

### Face

```text
head_base
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
brow_l
brow_r
mouth_neutral
```

左右眼绝不能烘焙成一张脸图。

### Hair

至少：

```text
hair_back_c
hair_back_l1
hair_back_l2
hair_back_r1
hair_back_r2

hair_side_l1
hair_side_r1

bang_l1
bang_c
bang_r1
```

长后发不能是一整块不可变形图片。

### Bow

至少独立：

```text
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

`bow_glow` 可以后续单独使用 Additive。

### Body / dress

手臂、手、腿、鞋和裙摆主要层都必须保持独立。

关节附近必须画隐藏延伸，不允许只在可见边界处切图。

## 3. 创建 Spine 工程

新建：

```text
pets/sena/spine/project/sena.spine
```

Setup Pose：

- root 位于双脚中点地面。
- +X 向屏幕右。
- +Y 向屏幕上。
- 轻微自然 A Pose。
- 身体接近正面。
- 双手放松。
- 头发、蝴蝶结、裙摆处于自然静止状态。

不要为了让绑定“更好做”而把角色改成僵硬 T Pose。

## 4. R3B 必需骨骼

机器合同位于：

```text
pets/sena/spine/settings/r3b_contract.json
```

第一份导出必须存在：

```text
root
body_root
hips
torso
chest
neck
head
face_root
eye_l
eye_r
bow_root
hair_back_root
skirt_root
```

建议第一轮同时把左右主手臂、腿和头发 root 一次建好，但它们暂时不阻塞 R3B。

## 5. Skin

必须创建：

```text
base
```

第一份导出只要求 `base` 能完整显示 Sena。

后续再扩展：

```text
dir_r
dir_l
outfit_default
prop_headphones
prop_laptop
expression_fx
```

不要在第一轮为了凑未来 Skin 而复制大量无意义 attachment。

## 6. Region / Mesh

第一轮原则：

Region 优先用于：

- 眼睛高光。
- 眉毛。
- 瞳孔。
- 小水晶。
- 不需要弯曲的小装饰。

Weighted Mesh 优先用于：

- 长发。
- 刘海。
- 袖子。
- 裙摆。
- 雪纺。
- 大蝴蝶结翼和尾带。

预算：

- 理想有效顶点 < 1500。
- 超过 1500 Asset Gate 会提示 warning。
- 约 2500 以上视为需要重新检查是否过度加密。

不要为了“看起来专业”给每张图片自动铺高密度网格。

## 7. Idle

正式名字：

```text
idle
```

要求：

- 建议 4–6 秒 loop。
- 不能一直左右大幅摇摆。
- 呼吸、头部、肩部运动都非常轻。
- 发尾、裙摆、蝴蝶结允许少量 authored follow-through。
- 首尾必须无明显跳变。
- 不要在 `idle` 中制作频繁眨眼；眨眼由 Track 3 独立驱动。

第一份 idle 的目的不是炫技，而是证明 Sena 在桌面上“安静待着也自然”。

## 8. 左右独立 Blink

必须创建：

```text
blink_l
blink_r
```

建议再同时创建：

```text
blink_both
```

规则：

- `blink_l` 只影响左眼相关 attachment / bone / deform。
- `blink_r` 只影响右眼。
- 不移动 head/root/body。
- 不把左右眼写进同一条不可拆动画。
- 闭眼形状必须像自然眼睑闭合，不是把眼睛纵向压扁。

Runtime 后续会让左右眼错开 20–90 ms，因此素材本身必须可独立播放。

## 9. 第一轮暂时不要做

先不要做：

```text
walk_l
walk_r
turn_l_to_r
turn_r_to_l
sit_enter
sit_idle
coding_enter
coding_idle
music_bob
sleep_enter
sleep_idle
cat_pickup
cat_carry_idle
```

这些等第一份 Sena 本体通过视觉和 Runtime Gate 后再进入。

## 10. 导出

第一轮建议同时导出开发 JSON 和 binary：

```text
pets/sena/spine/export/
  sena.json
  sena.skel
  sena.atlas
  sena.png
```

Runtime Release 最终使用 `sena.skel`。

导出后不要手工改 skeleton JSON 来“修”命名，问题必须回 Spine 源工程解决。

## 11. 自动验收

执行：

```powershell
cargo run --example sena_spine_asset_gate
```

必须通过：

- Spine 3.8.x。
- `base` skin。
- `idle`。
- `blink_l`。
- `blink_r`。
- R3B 必需骨骼。
- setup pose 可渲染。
- idle 中间帧可渲染。
- atlas PNG 全部存在。
- 顶点/triangle 数据合法。

如果只想看导出的真实内容：

```powershell
cargo run --example sena_spine_asset_gate -- --inventory-only
```

## 12. 人工视觉验收

机器 Gate 通过不等于美术通过。

最后必须人工看：

1. Setup Pose 原尺寸。
2. 420 px 高。
3. 350 px 高。
4. 320 px 高。
5. idle 连续播放至少 30 秒。
6. 左右眼分别连续触发。
7. 角色轮廓周围透明边。
8. 长发 / 裙摆 / 蝴蝶结 mesh 变形有没有折角和漏缝。

只有机器 Gate + 人工视觉 Gate 都通过，才允许正式 `pet.json` 从 Sprite 切到 Spine。
