# UI and Embedded Browser Debugging - 2026-07-11

## Reported symptoms

- The desktop UI rendered as multiple compressed layers with text wrapping one word per line.
- The native title bar and an in-app header duplicated top-level navigation.
- Embedded browser panes stayed black and did not appear to share browser authentication.

## Root causes

- WebKitGTK returned corrupt absolute CSS pixel metrics on the active high-DPI setup. Structural
  dimensions expressed in `px` collapsed visually, and `getBoundingClientRect()` returned negative,
  greatly inflated values.
- The browser overlay used those raw DOM values to position a native WebKit view. Invalid bounds
  kept the native view at its initial 1x1 allocation, leaving only the black HTML placeholder visible.
- Browser cookie import ran only when the persistent cookie file did not exist, so it became stale.
  Profile discovery also omitted common Flatpak locations.
- The native overlay passed pointer input through to the HTML layer, making a visible browser view
  non-interactive.
- `nix develop path:.` copied the complete workspace, including large generated artifacts, and
  contributed to exhausting the root filesystem during repeated release builds.

## Implemented fixes

- Consolidated the application into one flex shell and removed the duplicate in-app header.
- Replaced absolute pixel dimensions with relative `rem` sizing and set an accessible root UI scale.
- Added browser-bound normalization for normal and broken WebKit coordinate systems.
- Persisted and immediately reapplied bounds when browser tabs are created or switched.
- Retried native browser creation until the GTK overlay is available.
- Allowed the native WebKit view to receive pointer input.
- Replaced the full-window native event overlay with individually positioned WebView overlay children;
  browser content receives input without blocking sidebar, project, or pane controls.
- Stopped forcing hardware acceleration and added load/process failure logging.
- Refreshed cookies from the selected or default browser whenever the WebKit context starts.
- Read lock-safe Firefox SQLite snapshots including WAL state, preserved `HttpOnly`, and added
  Flatpak profile discovery.
- Included the GIO TLS backend required for HTTPS navigation in embedded WebKit views.
- Added runtime GIO discovery and explicit GnuTLS selection so direct, packaged, and non-Nix builds
  do not depend on the compiler shell preserving `GIO_EXTRA_MODULES`.
- Included the GStreamer base/good/bad and libav plugins required for WebKit media sinks, subtitles,
  and common website video codecs.
- Disabled WebKitGTK's DMA-BUF renderer for nested browser views and selected the non-accelerated
  shared-memory rendering path to prevent blank or partially painted website content.
- Deferred native WebView creation until the browser pane has valid bounds, then applied those bounds
  before the first navigation so responsive sites never initialize against a 1x1 viewport.
- Removed the Chromium user-agent spoof so sites target the embedded WebKit engine correctly.
- Normalized GTK's fixed-point Xft DPI before either WebKit view starts. KDE supplied `127795`,
  which WebKitGTK converted into negative CSS viewport dimensions and a `-1/48` device-pixel ratio;
  the 96-DPI baseline restores DPR 2 and matching GTK, inner, and client viewport dimensions.
- Removed the temporary 175% root-font workaround after correcting the underlying DPI calculation.

## Verification

- The corrected layout was visually captured with a full-width sidebar and readable centered content.
- The user confirmed the embedded browser fix worked.
- Unit suite: 28 passing tests, including browser geometry, lock-safe SQLite snapshot, and runtime
  GIO backend coverage.
- The release desktop binary built successfully in the Nix development environment.
- A clean-environment release launch resolved `libgiognutls.so`, initialized the browser context,
  and imported 989 Firefox cookies without a TLS error.

## Follow-up

- After `flake.nix` is tracked, prefer `nix develop .#` so Nix uses the Git-filtered source instead of
  copying generated files. Until then, `path:.` is required for Nix to see the untracked flake.
- Browser authentication sharing covers cookies. Browser passwords, extension state, passkeys, and
  arbitrary local storage are intentionally not copied.
