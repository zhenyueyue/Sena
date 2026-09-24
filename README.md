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
- Coding: 2-frame typing motion at 160 ms per frame.
- ListeningMusic: 4-frame low-frequency sway at 240 ms per frame.
- CodingWithMusic: combined typing and music motion at 160 ms per frame.
- Drowsy: 2-frame slow motion at 900 ms per frame.
- Sleeping: 2-frame breathing/Zzz motion at 1500 ms per frame.

There is intentionally no global 60 FPS ticker. Each behavior owns its own cadence, and static states stop animation scheduling entirely.

### Pet packages

Sena now loads character behavior metadata from `pets/<character>/pet.json`. Animation cadence, frame counts, looping behavior, renderer type, author/version metadata, asset paths, and asset license metadata live outside the executable.

The bundled `pets/default/pet.json` drives the current placeholder. Sprite packages can declare transparent PNG/WebP frame paths, and invalid/unsafe asset paths are rejected. If an external package is missing or broken, Sena safely falls back to a built-in placeholder configuration.

Set `SENA_PET_PACKAGE` to a package directory to override the default package during development. See [pets/README.md](pets/README.md) for the schema and directory layout.

The Sprite renderer is now connected end-to-end. PNG/WebP frames are decoded only when first used and then cached in memory, so animation ticks do not repeatedly read or decode files from disk. Missing or invalid frames safely fall back to the built-in placeholder.

Sprite packages now control character scale and use alpha-aware native Windows regions. The native window follows the source image size multiplied by the package scale, while transparent pixels are excluded from the window region so they do not block clicks to applications underneath.

The first official Sena character production spec now lives in [pets/sena](pets/sena): it defines the visual identity, fixed 768×1024 Sprite canvas, six initial behavior animations, frame cadence, alpha requirements, and the target `sena.official` package manifest.

The package runtime now prefers a valid official `pets/sena/pet.json` over the placeholder package. Sprite behaviors can also be produced incrementally: unfinished states fall back to that package's Idle frames so the official character stays visible.

The official `pets/sena/pet.json` is now active with the first real Sena + cat Idle Sprite. The initial runtime asset is a lightweight 256×341 transparent WebP prototype displayed at `sprite.scale = 0.9`; it keeps the full Sprite/alpha-hit-test pipeline testable while higher-resolution production animation frames are prepared.

The next asset milestone is deriving the remaining Idle frames from the locked reference, then replacing the lightweight prototype with the normalized 768×1024 production set without changing renderer code.

## License

Sena's source code is licensed under the [Apache License 2.0](LICENSE).

Character artwork, Live2D models, animation assets, fonts, and other third-party or bundled media may use separate licenses when explicitly stated. The Apache-2.0 license for the source code does not override those asset-specific terms.
