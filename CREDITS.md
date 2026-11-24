# Credits

StarPlay is free software, released under the GPL-3.0-or-later license. It is built on the following projects.

## Video engine (library shipped next to the executable)

| Project | Role | License | Credit line |
|---|---|---|---|
| [mpv](https://mpv.io) (libmpv) | Playback, decoding, Direct3D 11 / HDR rendering (`libmpv-2.dll`) | GPL-2.0-or-later (GPL build) | "Video playback: mpv (mpv.io), GPL-2.0-or-later" |
| [FFmpeg](https://ffmpeg.org) | Demuxing and decoding of every format (built into `libmpv-2.dll`) | GPL (`--enable-gpl` build) | "This software uses code of FFmpeg (ffmpeg.org) licensed under the GPL" |
| [mpv-winbuild-cmake](https://github.com/shinchiro/mpv-winbuild-cmake) (shinchiro) | Windows build of libmpv used here (`mpv-dev-x86_64-20261111-git-bcde6b93cc`) | GPL-3.0 (build scripts) | "libmpv for Windows built by shinchiro/mpv-winbuild-cmake" |

The libmpv DLL also embeds the libraries compiled by that project (libplacebo, dav1d, libass, etc.), each under its own free license; the exact list is the one published with the build version above on the mpv-winbuild-cmake releases page.

## Icon

StarPlay logo and icon (`logo/`): provided by the project author (license to be confirmed by the author).

## Rust crates used directly

| Crate | Role | License | Credit line |
|---|---|---|---|
| [slint](https://slint.dev) | User interface (transparent control window) | GPL-3.0-only, used under this option (Slint is also offered under royalty-free or commercial licenses) | "UI: Slint (slint.dev), GPL-3.0" |
| [libmpv2](https://github.com/kohsine/libmpv-rs) / libmpv2-sys | Rust bindings to the libmpv client and render APIs | LGPL-2.1 | "libmpv-rs (kohsine), LGPL-2.1" |
| [windows-sys](https://github.com/microsoft/windows-rs) | Win32 calls (video window, frameless window, fullscreen) | MIT or Apache-2.0 | "windows-rs © Microsoft, MIT" |
| [raw-window-handle](https://github.com/rust-windowing/raw-window-handle) | Win32 handle of the Slint window | MIT or Apache-2.0 or Zlib | "raw-window-handle, MIT" |
| [embed-resource](https://github.com/nabijaczleweli/rust-embed-resource) | Build only: embeds the icon and version info (`starplay.rc`) | MIT | "embed-resource (nabijaczleweli), MIT" |

## Indirect Rust crates

The ~310 crates pulled in by the ones above (survey of 2026-11-23) are all under free licenses compatible with GPL-3: MIT, Apache-2.0, BSD-2/3-Clause, Zlib, ISC, BSL-1.0, Unicode-3.0, 0BSD, CC0-1.0, Unlicense; Slint's internal crates (`i-slint-*`) follow Slint's license. The full list with licenses for the Windows target can be regenerated from the repository root:

```powershell
(cargo metadata --format-version 1 --filter-platform x86_64-pc-windows-msvc | ConvertFrom-Json).packages | Sort-Object name | ForEach-Object { "$($_.name) $($_.version) : $($_.license)" }
```

## Build tools

| Tool | Role | License |
|---|---|---|
| [NSIS](https://nsis.sourceforge.io) | Builds the Windows installer (`installer/starplay.nsi`) | zlib/libpng |
