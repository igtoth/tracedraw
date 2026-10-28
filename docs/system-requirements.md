# System requirements

What TraceDraw needs to run, for the builds published on the
[releases page](https://github.com/igtoth/tracedraw/releases). The
operating system, library and graphics lines come from the build
settings, the packaging files and the release programs themselves. The
memory, processor and disk lines are measured or estimated, and say so.

## At a glance

|                 | Minimum                                             | Recommended                                     |
|-----------------|-----------------------------------------------------|-------------------------------------------------|
| Windows         | Windows 10 version 1809, x64                        | Windows 10 22H2 or Windows 11, x64              |
| macOS           | macOS 11 Big Sur, Apple silicon or Intel            | A current macOS release                         |
| Linux           | x86_64 or arm64 with glibc 2.35 (Ubuntu 22.04, Debian 12, Fedora 36 or later) | A current distribution, X11 or Wayland |
| Browser         | WebGL 2                                             | A current Chrome, Edge, Firefox or Safari       |
| Graphics        | OpenGL 2.1, or none (drawn on the CPU)              | A card with Direct3D 12, Vulkan or Metal        |
| Memory (RAM)    | 2 GB (estimate)                                     | 8 GB (estimate)                                 |
| Processor       | Any 64-bit x86_64 or ARM64 processor, 2 cores       | 4 cores or more                                 |
| Disk            | 150 MB free                                         | 500 MB free                                     |

## Operating systems

- **Windows:** 10 version 1809 (build 17763) or later, x64. That is the
  floor of the Store package (`packaging/windows/msix/AppxManifest.xml`);
  the installer and the portable zip carry the C runtime inside the
  program, so a fresh Windows needs nothing more. There is no 32-bit
  build and no Windows on ARM build.
- **macOS:** 11 Big Sur or later (`LSMinimumSystemVersion` in
  `packaging/macos/Info.plist`), with a separate download for Apple
  silicon (arm64) and for Intel (x86_64).
- **Linux:** x86_64 and arm64, as AppImage, `.deb` or a plain `.tar.gz`.
  The programs are built on Ubuntu 22.04 and need **glibc 2.35 or later**
  (the newest symbol version in `objdump -T` of the 0.19.8 programs is
  `GLIBC_2.35`; the `.deb` asks for `libc6 (>= 2.35)`). The AppImage does
  not carry its own glibc, so it has the same floor. In practice:
  Ubuntu 22.04, Linux Mint 21, Debian 12, Fedora 36, RHEL, Alma or
  Rocky 10, or later.
  **Older distributions** (Ubuntu 20.04, Debian 11, RHEL 9 and its
  rebuilds, glibc 2.34 or older) cannot start the native programs; the
  [browser version](https://igtoth.github.io/tracedraw/) runs there.
  Other libraries are loaded when the program starts: `libxkbcommon`
  (and `libxkbcommon-x11` on X11), the system's OpenGL or EGL, and a
  Vulkan driver when there is one. A desktop that runs other graphical
  programs has them. The open and save dialogs use the desktop portal
  (`xdg-desktop-portal`) or, without it, `zenity`.
- **Browser:** a browser with WebGL 2 (the page says so when it is
  missing). On an Android tablet or an iPad the page can be installed
  as an app from the browser; a tablet with a mouse and keyboard, or
  the Touch workspace, suits editing best (`docs/behavior/web.md`,
  Installing on a tablet).

## Graphics

TraceDraw tries, in turn, until the window opens and shows what it
draws (`apps/tracedraw/src/gpu.rs`,
[`docs/behavior/about-and-startup.md`](behavior/about-and-startup.md)):

1. **The graphics card through wgpu:** Direct3D 12 or Vulkan on
   Windows, Vulkan on Linux, Metal on macOS. This is the fastest.
2. **OpenGL:** OpenGL 3.3 core or OpenGL ES first, then a compatibility
   context of any version. Below OpenGL 3.0 a simpler painter draws the
   window, so a card with **OpenGL 2.1** is enough: an office PC from
   2010 with Intel HD Graphics (Ironlake) under Windows 10 runs on its
   own card since 0.19.8 (#616).
3. **The CPU** (WARP on Windows, a CPU driver such as llvmpipe on
   Linux): it always opens, at 30 frames a second, without shadows and
   with the system pointers. Fine for light work, slow on large drawings.

So the minimum is OpenGL 2.1 for drawing on the card, or no usable card
at all. The recommended setup is a card whose current driver offers
Direct3D 12, Vulkan or Metal. `tracedraw --renderer=gl` or
`--renderer=software` starts with a given renderer, and `tracedraw.log`
in the settings folder names the one in use.

## Memory, processor and disk

These are measurements of 0.19.8 on Linux x86_64 (4 cores, a
1920 x 1080 screen, drawn on the CPU) with `sample.tdraw` open, and
estimates from them:

- **Memory:** about 170 MB with the card or OpenGL and about 220 MB on
  the CPU renderer, a few seconds after the window opened (resident set
  size). Large drawings, bitmaps and effects (blends with many steps,
  contours, exports of big rasters) take more. Estimate: 2 GB in the
  computer is the minimum with the system, 8 GB is comfortable for big
  `.cdr` files and exports.
- **Processor:** start-up used about 3 seconds of processor time. No
  instruction set beyond the base of x86_64 or ARM64 is needed. Estimate:
  2 cores are enough on a graphics card; with no usable card the CPU
  draws every frame, and 4 cores or more keep it smooth.
- **Disk:** the programs take about 100 MB (`tracedraw` 82 MB and
  `tracedraw-cli` 17 MB on Linux; the downloads are 20 to 37 MB). Settings,
  the log and autosaves take a few MB more. The recommended figure leaves
  room for autosaves of large documents and exports.

Something here wrong for your computer? Send `tracedraw.log` in
[an issue](https://github.com/igtoth/tracedraw/issues/new?template=bug_report.yml).
