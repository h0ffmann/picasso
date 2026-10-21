# present

The desk during a talk or a demo: draw over whatever is on screen, capture or record it, keep
the webcam looking the same as last time, and present from a PDF or a markdown file. One
Rust program, `cam`, for the part nothing packaged does well — saving and restoring V4L2 controls —
and otherwise a list of tools with `just` recipes that pick Wayland or X11 for you.

| Do | Tool | Recipe |
|---|---|---|
| Draw / highlight over any window | [gromit-mpx](https://github.com/bk138/gromit-mpx) — F9 draw, Shift+F9 hide, F10 clear | `just annotate` |
| Show the cursor, show keystrokes | [find-cursor](https://github.com/arp242/find-cursor) · [screenkey](https://gitlab.com/screenkey/screenkey) (X11) · [wshowkeys](https://git.sr.ht/~sircmpwn/wshowkeys) (Wayland) | — |
| Screenshot + annotate | [flameshot](https://flameshot.org/) (X11 / portal) · [grim](https://sr.ht/~emersion/grim/) + [slurp](https://github.com/emersion/slurp) + [satty](https://github.com/gabm/Satty) (Wayland) | `just shot` |
| Record the screen | [wf-recorder](https://github.com/ammen99/wf-recorder) (Wayland CLI) · [kooha](https://github.com/SeaDve/Kooha) (Wayland GUI) · [ffmpeg](https://ffmpeg.org/) x11grab · [OBS Studio](https://obsproject.com/) for webcam overlays and streaming | `just rec [region]` |
| mp4 → gif | ffmpeg + [gifski](https://gif.ski/) | `just gif file.mp4` |
| Webcam: brightness, zoom, focus, exposure | [v4l-utils](https://linuxtv.org/wiki/index.php/V4l-utils) (`v4l2-ctl`) · [cameractrls](https://github.com/soyersoyer/cameractrls) (GUI, presets) · [guvcview](https://guvcview.sourceforge.net/) (preview) | `just cam …`, `just cam-gui` |
| Present a PDF with notes and a timer | [pdfpc](https://pdfpc.github.io/) | `just pdf deck.pdf` |
| Present markdown in the terminal | [presenterm](https://github.com/mfontanini/presenterm) | `just md deck.md` |

```console
just shell            # everything on PATH, in your current directory
just self-test        # cam's cargo tests (CI builds them as the cam check)
just versions         # what is pinned, built in the sandbox
```

## `cam` — webcam controls you can save and get back

Webcams forget brightness, zoom and manual focus on unplug and on suspend, and every camera names
its controls differently, so the thing you want two minutes before a call is "the settings from
last time".

```console
cam list                                      # devices and their /dev/video* nodes
cam show                                      # every control with a value: current, default, flags
cam set brightness=140 zoom_absolute=120      # one v4l2-ctl call per control; a bad one names itself
cam set focus_automatic_continuous=0 focus_absolute=30
cam save meeting                              # → ~/.config/present/cam/meeting.conf, auto-* lines first
cam load meeting                              # replay it (after a replug, a reboot, a suspend)
cam reset                                     # every writable control back to its default
CAM_DEVICE=/dev/video2 cam show               # or: cam show /dev/video2
```

Profiles are plain `name=value` lines; edit them. Read-only and inactive controls are shown by
`show` but never saved or reset. `load` applies `*auto*` controls first because most cameras
reject a manual focus or exposure while the automatic one is still on. `v4l2-ctl` is resolved
from `PATH` rather than wrapped in, so the tests can fake it — building `cam` runs them, and that build is
`checks.<system>.cam`.

## Linux notes

- Wayland vs X11 is decided per recipe by `$WAYLAND_DISPLAY`. GNOME on Wayland: `grim` needs a
  wlroots compositor; use `flameshot gui` (portal) or GNOME's own `Print` there. `wf-recorder`
  likewise; `kooha` and OBS work through the portal on any compositor.
- A virtual camera (OBS "Start Virtual Camera", a cropped or filtered feed into Meet) needs the
  `v4l2loopback` kernel module, which a Nix shell cannot provide on Ubuntu:
  `sudo apt install v4l2loopback-dkms`, then OBS finds it.
- GNOME's built-in zoom (Settings → Accessibility → Zoom, or `Super`+`Alt`+`8`) and a large
  cursor cover most "can you see that?" moments without any of this.

## macOS

`aarch64-darwin` gets the terminal half only — `presenterm`, `ffmpeg`, `gifski` — because the
desktop tools here are Linux, and the macOS equivalents (Presentify, Ink2Go, Keycastr) are not
in nixpkgs. Built-in zoom (`Ctrl`+scroll with Accessibility → Zoom on) plus "shake to enlarge
cursor" is the zero-install answer there.

## Reusing it from another repository

```nix
{
  inputs.present = { url = "github:h0ffmann/picasso?dir=labs/present"; inputs.nixpkgs.follows = "nixpkgs"; };
  outputs = { nixpkgs, present, ... }: {
    devShells.x86_64-linux.default = nixpkgs.legacyPackages.x86_64-linux.mkShell {
      packages = [ /* the project's own tools */ ] ++ present.lib.x86_64-linux.tools;
    };
  };
}
```

`lib.<system>.tools` is the list (desktop tools only on Linux), `lib.<system>.scripts.cam` and
`packages.<system>.cam` the program, `packages.<system>.<tool>` each tool for `nix run`, and
`checks.<system>.{cam, versions}` what `nix flake check` builds.

## What it pins

| Type       | Program |
| :--------- | :-----: |
| Annotate   | [gromit-mpx](https://github.com/bk138/gromit-mpx), [find-cursor](https://github.com/arp242/find-cursor), [screenkey](https://gitlab.com/screenkey/screenkey) / [wshowkeys](https://git.sr.ht/~sircmpwn/wshowkeys) |
| Capture    | [flameshot](https://flameshot.org/), [grim](https://sr.ht/~emersion/grim/) + [slurp](https://github.com/emersion/slurp) + [satty](https://github.com/gabm/Satty) |
| Record     | [wf-recorder](https://github.com/ammen99/wf-recorder), [kooha](https://github.com/SeaDve/Kooha), [OBS Studio](https://obsproject.com/), [ffmpeg](https://ffmpeg.org/) + [gifski](https://gif.ski/) |
| Webcam     | [v4l-utils](https://linuxtv.org/wiki/index.php/V4l-utils) + `cam` (save / load / reset profiles), [cameractrls](https://github.com/soyersoyer/cameractrls), [guvcview](https://guvcview.sourceforge.net/) |
| Present    | [pdfpc](https://pdfpc.github.io/), [presenterm](https://github.com/mfontanini/presenterm) |
| Reuse      | `lib.<system>.{tools, scripts}`, `packages.<system>.{cam, <tool>}`, `checks.<system>.{cam, versions}` |
