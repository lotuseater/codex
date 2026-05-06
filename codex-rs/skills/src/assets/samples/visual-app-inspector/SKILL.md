---
name: visual-app-inspector
description: Use when Codex needs to inspect, test, or control a live GUI, browser, desktop app, terminal window, screenshot-backed workflow, or app-specific automation harness. Prefer app-native harnesses first, then built-in native DAB tools, with Wizard DAB only as a compatibility fallback.
---

# Visual App Inspector

Use this skill for live GUI/runtime work, including screenshots, visual defect checks, browser clicking, desktop app testing, terminal or PowerShell window inspection, and reproductions that depend on what is visibly on screen.

## Provider Order

1. Run `automation_harness_detect` for repo/app work when a harness may exist.
2. Use the app-native or repo visual harness if detected.
3. Use built-in native DAB tools for outer desktop control:
   - `dab_find_window`
   - `dab_window_check`
   - `dab_screenshot`
   - `dab_ocr`
   - `dab_visual_scan`
   - `dab_element_map`
   - `dab_navigate`
   - `dab_smart_click`
   - `dab_click`
   - `dab_bg_click`
   - `dab_send_keys`
4. Use Wizard DAB only when native DAB is unavailable or the user explicitly needs Wizard-specific state.
5. Use plain logs or filesystem artifacts only after live visual surfaces are unavailable or irrelevant.

## Workflow

- Inspect before acting: find or check the window, then scan or screenshot it.
- Prefer `dab_smart_click` or element coordinates from `dab_element_map` over blind coordinates.
- For GUI input, validate the target window immediately before sending clicks or keys.
- Capture evidence after GUI input with `dab_visual_scan` or `dab_screenshot`.
- For browser tasks, prefer Playwright or app-local browser harnesses when the repo provides them; otherwise use native DAB.
- For terminals, PowerShell, Calculator, Notepad++, Chrome, Paint, and similar Windows apps, native DAB is the default desktop surface.
