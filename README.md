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
5. Animation frame rate is chosen per animation instead of globally.
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

The next milestone is external pet-package assets (Sprite/WebP frames), tray/settings controls, and position/configuration persistence.

## License

Sena's source code is licensed under the [Apache License 2.0](LICENSE).

Character artwork, Live2D models, animation assets, fonts, and other third-party or bundled media may use separate licenses when explicitly stated. The Apache-2.0 license for the source code does not override those asset-specific terms.
