# Tablets and the Android app

TraceDraw runs on tablets in two ways: the browser version installed as an
app (on an Android tablet, an iPad or a desktop computer), and a native app
for Android tablets, phones and Chromebooks. Both are the same editor as the
desktop version, with the same menus, tools and files.

## Installing the browser version as an app {#install}

The [browser version](https://igtoth.github.io/tracedraw/) can be installed
from the browser. It then opens full screen with the TraceDraw icon, without
the browser's address bar and tabs, in any orientation, and keeps clear of a
notch or rounded screen corners.

- **Android tablet** (Chrome, Edge, Samsung Internet): open the page, then
  the browser's menu and **Install app** or **Add to Home screen**. Chrome
  may also offer it in a bar under the address bar.
- **iPad** (Safari): open the page, tap **Share**, then **Add to Home
  Screen**.
- **Desktop** (Chrome, Edge): click the install icon at the right end of the
  address bar, or choose **Install TraceDraw** in the browser's menu.

Once it has been opened, the installed app starts without a network
connection, at the version you last used (fonts fetched from the web still
need a connection the first time your text uses them). A new version is taken
the next time the app starts after all its pages and windows were closed.

It is the same program as the browser version, so the differences listed in
[The browser version](browser-version.md#differences) apply.

## The Android app {#android}

The Android app is TraceDraw built for 64-bit ARM tablets and phones and for
Chromebooks with Intel or AMD processors. Its installation files (APK) are
attached to the releases and to the nightly builds of the project on GitHub.
The app does not check for updates itself; [[menu.help > menu.help.updates]]
opens its store page.

A new installation starts in the [[menu.window.workspace_touch]] workspace:
larger bars and buttons, no rulers, panels or status bar; two fingers pan and
a pinch zooms. [[menu.window > menu.window.workspace]] picks another one, and
the choice is kept. See [Workspaces](../customize/workspaces.md).

What works differently:

- **Files.** Open, Import, Save, Save As, Export and Publish to PDF go
  through the system's document picker. Save writes an opened document back
  in place. The [[dialog.save_drawing]] dialog box asks only for the name,
  type and version; the system then asks where to save.
- **Recent drawings** reopen from the list and the Welcome Screen, as on the
  desktop.
- **Clipboard.** Text goes through the system clipboard: objects copied in
  TraceDraw paste in other apps as text, and text copied elsewhere pastes as
  text. Pictures copied in other apps are not read yet.
- **Fonts.** The system's fonts are used.
- **Keyboard.** A physical or Bluetooth keyboard works with the shortcuts.
- **Help** pages open in the browser.

Not available on Android: printing, Send To, editing a bitmap in another
program, and choosing a folder (such as an export folder or a font folder).

## Touch and pen {#touch}

On a touch screen, a finger or a pen works the canvas, the toolbox, the bars
and the menus. Tap to click, drag to draw or move. In the
[[menu.window.workspace_touch]] workspace two fingers pan and a pinch zooms.
For pen pressure on desktop systems, see
[Drawing with a graphics tablet](../lines-shapes/pen-pressure.md).
