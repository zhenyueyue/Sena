# Sena Spine 动画与状态合同 v1

本文件定义 Spine 工程与 Rust 行为系统之间的稳定接口。

原则：

> 行为系统只说“星奈现在想做什么”，Spine Director 决定“用哪些动画、Skin、Track 和过渡把它表现出来”。

## 1. Setup Pose

Setup pose 使用轻微自然 A Pose，而不是僵硬 T Pose。

约束：

- root 位于双脚中点的地面基线。
- +X = 屏幕右。
- +Y = 屏幕上。
- setup pose 不携带任何情绪。
- 双手放松。
- 双脚平行或轻微外开。
- 头发 / 裙摆处于自然静止位置。
- 猫不嵌在 Sena setup pose。

## 2. AnimationState Track 约定

### Track 0 — Base / Posture

拥有：

- root
- body_root
- hips
- torso
- arms
- legs
- 主头部姿态
- 坐 / 站 / 睡等整体姿态

典型动画：

- idle
- walk
- turn
- sit
- coding posture
- sleeping posture

### Track 1 — Context / Upper-body Overlay

只允许 key 自己负责的骨骼。

用途：

- typing
- music bob
- petting
- 小幅手部互动
- 耳机/电脑相关上身动作

如果一个动作会改变腿部或 root，就不应该放 Track 1。

### Track 2 — Expression

只 key：

- brow
- mouth
- cheek / face mesh small deform
- expression-specific eye shape

不修改头部位置和身体。

### Track 3 — Micro

只 key：

- blink
- eye look
- very small face micro motion

Blink 左右眼可独立。

## 3. Key Ownership

同一属性尽量只由一个 Track 层级负责。

例如：

- `head.rotation`：Track 0。
- `mouth`：Track 2。
- `eyelid_l`：Track 3。
- `forearm_l`：Track 0，除非一个明确允许叠加的 upper-body overlay。

禁止做一个“万能动画”同时 key 全身所有骨骼，这会让混合失效。

## 4. 正式动画名

命名全小写 snake_case。

### 4.1 Idle

必需：

```text
idle
idle_sway
idle_look_l
idle_look_r
```

建议：

- `idle`：4–6 秒无缝 loop。
- `idle_sway`：1.5–2.5 秒 one-shot。
- `idle_look_l/r`：0.8–1.4 秒 one-shot。

Idle 不要每秒都晃。角色应有“安静时间”。

### 4.2 Face / Micro

```text
blink_l
blink_r
blink_both
expr_happy
expr_curious
expr_sleepy
expr_focused
expr_surprised
```

要求：

- `blink_l` 和 `blink_r` 必须能单独播放。
- 正常随机眨眼用两眼相差 20–90 ms 的错峰，而不是永远同帧。
- 猫使用独立随机时钟。

### 4.3 Locomotion

```text
walk_l
walk_r
turn_l_to_r
turn_r_to_l
walk_stop_l
walk_stop_r
```

建议：

- walk：0.7–0.9 秒 loop。
- turn：0.35–0.55 秒。
- stop：0.15–0.3 秒。

正式左右朝向使用独立方向 attachment/Skin，不用整角色镜像作为最终方案。

### 4.4 Sit

```text
sit_enter
sit_idle
sit_exit
```

建议：

- enter：0.4–0.6 秒。
- idle：3–5 秒 loop。
- exit：0.4–0.6 秒。

`sit_enter` 最后一帧必须与 `sit_idle` setup 对齐。

### 4.5 Coding

```text
coding_enter
coding_idle
coding_type
coding_exit
```

行为：

- `coding_enter`：拿出 / 打开电脑并进入坐姿。
- `coding_idle`：不打字时保持自然小动作。
- `coding_type`：Track 1 overlay，只 key 手臂 / 手指 / 小幅上身。
- 键盘活动时开启 `coding_type`。
- 停止输入后 fade out 到 `coding_idle`，不要让整套 coding 动画一直高速循环。

### 4.6 Listening

```text
listening_enter
listening_idle
music_bob
listening_exit
```

行为：

- enter 中戴上耳机。
- `music_bob` 是 Track 1 小幅 overlay。
- 音乐停止时播放 exit 并摘下耳机。
- 不允许“耳机瞬间出现”。

### 4.7 Coding + Music

不制作一份完整重复的 `coding_with_music_full`。

组合：

```text
Track 0: coding_idle
Track 1: coding_type / music_bob（根据活动选择或小幅组合）
Skin: prop_laptop + prop_headphones
Track 2/3: face overlays
```

这样避免复制大量时间线。

### 4.8 Drowsy / Sleep

```text
drowsy_enter
drowsy_idle
sleep_enter
sleep_idle
sleep_exit
```

要求：

- drowsy 先出现困倦信号，再真正睡。
- sleep_enter 必须有完整“重心下降 -> 闭眼 -> 安定”过程。
- 不能从站立一帧切成睡觉。
- sleep_idle 保持非常低频运动。
- 唤醒播放 sleep_exit。

### 4.9 Interaction

```text
petting
stretch
look_at_cat
daydream
click_react_01
click_react_02
drag_react
```

其中：

- `stretch` 改变全身姿态 -> Track 0 one-shot。
- `petting` 只有在当前 posture 兼容时可走 Track 1。
- `daydream` 可以 Track 0，结束后回当前基础状态。
- 点击反应优先短，约 0.25–0.7 秒。

### 4.10 Cat Carry

Sena：

```text
cat_pickup
cat_carry_idle
cat_carry_walk_l
cat_carry_walk_r
cat_putdown
```

Cat：

```text
cat_idle
cat_walk
cat_pickup_react
cat_carry_idle
cat_putdown_react
cat_sleep
```

## 5. Spine Events

动画内只放语义事件。

正式事件名：

```text
facing_l
facing_r

foot_l
foot_r

headphones_on
headphones_off

laptop_on
laptop_off

cat_attach
cat_detach

sleep_committed
wake_committed
```

语义：

- `facing_l/r`：Runtime 切换方向 Skin / draw-order state。
- `cat_attach`：猫 root 开始跟随 Sena `cat_carry` bone。
- `cat_detach`：猫从 carry bone 释放并开始落地过渡。
- props 事件控制 attachment，不靠动画结束时间猜测。

## 6. Mix Duration

默认混合：

| From | To | Mix |
|---|---|---:|
| idle | walk | 0.12 s |
| walk | idle | 0.12 s |
| idle | turn | 0.06 s |
| turn | walk | 0.08 s |
| idle | one-shot interaction | 0.10 s |
| expression | expression | 0.10 s |
| blink | none | 0.04 s |
| context overlay | none | 0.10 s |

姿态跨度很大的变化尽量使用显式 transition 动画，而不是把 mix 拉到 0.5 秒去“糊过去”。

## 7. Loop Seam

所有 loop 必须：

- 起始/结束 root 一致。
- 关键骨骼旋转连续。
- 二级运动 fallback 在 loop seam 连续；运行时 spring reset 后不能爆跳。
- draw order 不无故改变。
- attachment 状态一致。

不要用最后一帧复制第一帧来伪造 loop；曲线切线也必须连续。

## 8. 动画 FPS

Spine 时间线是连续时间，不要求 Runtime 按制作 FPS 播放。

推荐制作：

- 主动作：30 FPS timeline reference。
- 快速 click reaction：30/60 均可。
- Runtime 正常按 delta time 插值。
- Active render 可 60 FPS。
- Idle/低运动场景 Runtime 可降低刷新率。

## 9. 现有 Rust Behavior 映射

当前行为系统映射：

| Rust Behavior | Spine Director |
|---|---|
| Idle | `idle` + random idle one-shots |
| Coding | `coding_enter -> coding_idle`，typing 时 Track1 `coding_type` |
| ListeningMusic | `listening_enter -> listening_idle` + `music_bob` |
| CodingWithMusic | `coding_idle` + laptop/headphones + context overlays |
| Drowsy | `drowsy_enter -> drowsy_idle` |
| Sleeping | `sleep_enter -> sleep_idle` |

现有 interaction：

| Interaction | Spine |
|---|---|
| Petting | `petting` |
| Stretch | `stretch` |
| LookAtCat | `look_at_cat` |
| Daydream | `daydream` |

## 10. 状态切换原则

Runtime 维护“目标语义状态”，不是粗暴 set animation。

例：

```text
Idle
 -> user starts coding
 -> coding_enter
 -> laptop_on event
 -> coding_idle
 -> keyboard activity
 -> coding_type overlay
 -> typing stops
 -> fade overlay
 -> coding_idle
```

音乐：

```text
Idle
 -> media starts
 -> listening_enter
 -> headphones_on event
 -> listening_idle + music_bob
 -> media stops
 -> listening_exit
 -> headphones_off
 -> Idle
```

睡眠：

```text
Idle
 -> Drowsy
 -> drowsy_enter
 -> drowsy_idle
 -> Sleeping
 -> sleep_enter
 -> sleep_committed
 -> sleep_idle
```

## 11. 打断规则

高优先级打断：

1. 用户拖拽。
2. 双击 / 直接互动。
3. 桌面位置安全修正。
4. 唤醒。
5. 行为上下文变化。
6. 自主小动作。

不可打断区间可由动画事件/Runtime flag 标记，例如：

- 猫刚离地到安全抱稳。
- 坐下重心尚未落稳。
- sleep_enter 最后 150 ms。

但不可打断窗口要短，不能让桌宠“卡住不理人”。

## 12. 验收

每个正式动作必须同时过：

- 100% 速度。
- 50% 慢放。
- 320 px 角色高度。
- 420 px 角色高度。
- 左右朝向。
- 动画开始/结束前后混合。
- 点击拖拽打断测试。

只有“单独看动画很漂亮”不算通过，必须在状态机里也漂亮。
