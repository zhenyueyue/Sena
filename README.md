# Sena（星奈）

Sena is a lightweight, context-aware Windows desktop companion inspired by a Japanese anime-style desktop character.

She reacts to what you are doing: coding, listening to music, stepping away from the PC, or locking Windows. The long-term rendering path supports both lightweight sprite pets and richer Live2D characters.

## Technical direction

- Rust owns desktop context, behavior decisions, persistence, and Windows integration.
- Slint owns ordinary UI and the initial lightweight pet surface.
- Pet behavior is semantic and renderer-independent so a future Live2D backend does not require rewriting the context or behavior engines.
- Windows integrations should prefer event notifications over polling.

## Performance rules

1. No permanent 60 FPS loop.
2. No high-frequency full process scans.
3. Idle means event-driven sleep whenever possible.
4. Foreground-app changes, media changes, session changes, and input-idle changes should use OS events or low-frequency fallbacks.
5. Animation cadence is chosen per animation and may be overridden per frame for natural low-duty-cycle motion such as blinking.
6. Expensive renderers such as Live2D are activated only while needed.
7. React/WebView/Electron are intentionally not part of the pet runtime.

## Initial architecture

```text
src/
├─ context/      # What is happening on the desktop?
├─ behavior/     # What should the pet do?
├─ render/       # How should that behavior be drawn?
└─ main.rs

ui/
└─ pet.slint
```

The current UI is intentionally a tiny placeholder.

### Implemented context signals

- Windows foreground application changes via SetWinEventHook(EVENT_SYSTEM_FOREGROUND).
- Initial foreground process lookup on startup.
- Coding behavior mapping for Codex, VS Code, Cursor, IntelliJ IDEA, CLion, and RustRover.
- Windows Global System Media Transport Controls playback state.
- Windows lock/unlock session notifications through WTS session events.
- User idle duration through GetLastInputInfo.
- Windows low-level keyboard activity pulses for input-driven Coding; Sena does not retain key codes, text, scan codes, or modifier state.
- Live transitions between Idle, Coding, ListeningMusic, CodingWithMusic, Drowsy, and Sleeping.

Foreground, media, and lock/unlock observation are event-driven. User idle time is the only sampled signal and is checked once every 5 seconds with a single GetLastInputInfo call. The UI is only updated when the activity level actually changes.

Idle behavior defaults to 5 minutes for Drowsy and 10 minutes for Sleeping. Active media playback prevents input-idle sleep so passive listening/watching is not incorrectly treated as user absence. Locking Windows always forces Sleeping immediately.

### Implemented desktop interaction

- Drag the visible pet body with the left mouse button.
- Releasing the pet leaves it exactly where the user placed it; there is no default gravity or throw inertia.
- Dragging is constrained to the active monitor work area, including the taskbar boundary.
- There is no permanent physics or movement timer.
- The native Windows region is clipped to the visible pet body, so transparent corner pixels do not block clicks to applications underneath.

Gravity or playful throw physics may be added later as an explicit optional mode rather than default behavior.

### Animation runtime

Behavior is mapped to renderer-independent animation clips before the current Slint placeholder draws anything. The placeholder now demonstrates the same scheduling rules a future sprite or Live2D backend will consume:

- Idle: static, no permanent timer.
- Coding: static focused pose while the keyboard is quiet; short typing bursts only around real key activity.
- ListeningMusic: static between occasional one-shot music-motion bursts; the current rest pattern is 7 / 11 / 9 / 13 seconds with a short 900 ms motion window.
- CodingWithMusic: prefers dedicated combined assets when available, otherwise reuses Coding assets before falling back further.
- Drowsy: static sleepy pose between occasional 1.6-second three-frame motion bursts; current rest pattern is 18 / 27 / 22 / 31 seconds.
- Sleeping: completely static while Windows is locked; when unlocked after 10 minutes of inactivity, occasional 2.4-second three-frame breathing/Zzz bursts use a 35 / 52 / 43 / 61 second rest pattern.

There is intentionally no global 60 FPS ticker. Each behavior owns its own cadence, and static states stop animation scheduling entirely.

### Pet packages

Sena now loads character behavior metadata from `pets/<character>/pet.json`. Animation cadence, frame counts, looping behavior, renderer type, author/version metadata, asset paths, and asset license metadata live outside the executable.

The bundled `pets/default/pet.json` drives the current placeholder. Sprite packages can declare transparent PNG/WebP frame paths, and invalid/unsafe asset paths are rejected. If an external package is missing or broken, Sena safely falls back to a built-in placeholder configuration.

Set `SENA_PET_PACKAGE` to a package directory to override the default package during development. See [pets/README.md](pets/README.md) for the schema and directory layout.

The Sprite renderer is now connected end-to-end. PNG/WebP frames are decoded only when first used and then cached in memory, so animation ticks do not repeatedly read or decode files from disk. Missing or invalid frames safely fall back to the built-in placeholder.

Sprite packages now control character scale and use alpha-aware native Windows regions. The native window follows the source image size multiplied by the package scale, while transparent pixels are excluded from the window region so they do not block clicks to applications underneath.

The first official Sena character production spec now lives in [pets/sena](pets/sena): it defines the visual identity, fixed 768×1024 Sprite canvas, six initial behavior animations, frame cadence, alpha requirements, and the target `sena.official` package manifest.

The package runtime now prefers a valid official `pets/sena/pet.json` over the placeholder package. Sprite behaviors can also be produced incrementally: unfinished states fall back to that package's Idle frames so the official character stays visible.

The official `pets/sena/pet.json` now ships the first complete four-frame Sena + cat Idle loop on a normalized 768×1024 transparent canvas. The cat and Sena blink on different frames, with per-frame timing of 1800 / 120 / 2300 / 120 ms so closed-eye frames read as brief natural blinks rather than synchronized sleepiness.

Runtime decoding downsamples high-resolution Sprite sources to the actual physical display size before caching them. This preserves the 768×1024 production source while avoiding the memory cost of keeping every full-resolution RGBA frame resident when Sena is displayed much smaller on the desktop.

The official package now also ships a four-frame Coding v1 set. Coding is triggered by the real foreground-process signal and no longer falls back to Idle when a supported IDE is active.

High-resolution Sprite sources are downsampled to their physical display size before caching. Alpha hit regions are reused across frames within the same behavior instead of being rebuilt on every animation tick.

Coding is now input-aware. When a supported IDE is foreground but the keyboard is quiet, Sena freezes on the focused Coding pose with no animation timer. A real key-down pulse starts the Coding animation and keeps it active for 650 ms after the most recent keyboard activity; subsequent key activity extends that burst. Leaving the IDE or locking Windows cancels the burst immediately. The watcher is event-driven and only emits activity pulses; it does not record which key was pressed.

On the development machine, the static Coding pose measured effectively zero CPU over a 12-second sample. Continuous multi-frame Coding remains intentionally limited to the short periods in which the user is actively typing.

ListeningMusic now follows the same low-duty-cycle principle and the official package ships a four-frame Listening v1 set. Media playback itself does not justify a permanent animation loop: Sena stays on listening/000 most of the time and wakes for brief 240 / 220 / 240 / 220 ms motion bursts at staggered 7 / 11 / 9 / 13 second rest intervals. Stopping media or locking Windows cancels pending music motion immediately. CodingWithMusic currently reuses the finished Coding visual set, so typing remains input-driven even while music is playing.

Drowsy now follows the same event-sleep model and the official package ships a three-frame Drowsy v1 set. After 5 minutes without user input, Sena stays on drowsy/000 most of the time and wakes for short 420 / 760 / 420 ms sleepy-motion bursts after staggered 18 / 27 / 22 / 31 second rests. Real input, media playback, session lock, or the 10-minute transition to Sleeping cancels pending Drowsy motion immediately.

Sleeping is now prepared for the same low-duty-cycle runtime. If the desktop remains unlocked after 10 minutes of inactivity and dedicated Sleeping assets exist, Sena stays on sleeping/000 and only wakes for short 700 / 1000 / 700 ms breathing/Zzz bursts after staggered 35 / 52 / 43 / 61 second rests. Locking Windows cancels all pending Sleeping motion and keeps the pose completely static, because the desktop is not visible anyway. Until Sleeping art is shipped, the official package remains static in its Idle fallback.

## License

Sena's source code is licensed under the [Apache License 2.0](LICENSE).

Character artwork, Live2D models, animation assets, fonts, and other third-party or bundled media may use separate licenses when explicitly stated. The Apache-2.0 license for the source code does not override those asset-specific terms.
