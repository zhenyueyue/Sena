# Sena icons

Place the finalized Windows icon assets in this directory:

- `app.ico` — the full Sena + cat cartoon icon used by the executable, taskbar and Alt+Tab.
- `tray.ico` — the simplified star-emblem icon used only in the Windows notification area.

Both files should be multi-resolution ICO files. Recommended sizes are 16, 24, 32, 48, 64, 128 and 256 px for `app.ico`, and 16, 20/24, 32 and 48/64 px for `tray.ico`.

The build script embeds `app.ico` as resource ID 1 and `tray.ico` as resource ID 2. If either asset is missing, the application falls back gracefully instead of failing the build.
