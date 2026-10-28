# Microsoft Store listing

What to type in Partner Center for TraceDraw, field by field, so a
submission takes minutes. The package itself comes from the build (see
[`code-signing.md`](code-signing.md), Microsoft Store). Limits in
brackets are the Store's.

## Pricing and availability

- Markets: all.
- Pricing: Free.
- Visibility: public.

## Properties

- Category: Photo & video (or Productivity).
- Privacy policy URL: <https://github.com/igtoth/tracedraw/blob/main/PRIVACY.md>
- Website: <https://github.com/igtoth/tracedraw>
- Support contact: <https://github.com/igtoth/tracedraw/issues>
- Product declarations: none of the data, accessibility or
  inking boxes apply; the app works with a mouse and a keyboard.
- System requirements: Windows 10 version 1809 or later, x64. A
  graphics card with Direct3D 12, Vulkan or OpenGL 2.1 or later is used
  when present; without one the window is drawn on the CPU (see
  `docs/system-requirements.md`).

## Age ratings

Answer the questionnaire for an "Utility / productivity" app: no violence,
no sexual content, no gambling, no user-to-user communication, no sharing
of location or personal data, no purchases. The usual result is 3+ / E.

## Store listing (English, United States)

**Product name**: TraceDraw

**Short description** [up to 1000 characters, shown first]:

> Open-source vector editor for illustration, page layout and print.
> Pages and layers, curves and nodes, artistic and paragraph text, fills
> and outlines, live effects, symbols, PDF/X export and `.cdr` files, in
> a native program that starts fast and works offline. Alpha: expect
> bugs, and please report them.

**Description** [up to 10000 characters]:

> TraceDraw is a free, open-source vector editor for illustration, page
> layout and print, written from scratch in Rust.
>
> Draw with rectangles, ellipses, polygons, stars, spirals, 71 common
> shapes, freehand, Bezier, pen, B-spline and calligraphic brush tools.
> Edit curves node by node, weld, trim and intersect shapes, and round,
> scallop or chamfer corners one by one.
>
> Set artistic and paragraph text with real fonts, text on a path, columns,
> tabs, bullets and drop caps, and fit paragraphs around objects.
>
> Fill with solid colours, linear, radial, conical and square gradients,
> patterns, textures and mesh fills; give outlines any width, dash and
> arrowhead. Work in RGB or CMYK with colour palettes and colour styles.
>
> Live effects stay editable: blends, contours, drop and block shadows,
> envelopes, extrusions, bevels, lenses, transparency and distortions.
>
> Lay out multi-page documents with layers, master layers, guides at any
> angle, grids and snapping, then print with imposition, printers' marks
> and separations, or export PDF (PDF/X-1a, X-3, X-4), SVG, EPS, DXF, EMF,
> PNG, JPEG, WebP and more. Opens and saves `.cdr` files, and imports PDF,
> AI, SVG, EPS, DXF, PSD, EMF and WMF.
>
> Make it yours: dock the toolbox, palette and bars on any side, set your
> own keyboard shortcuts, keep objects on the Shelf for any drawing, and
> export your workspace to another computer. The interface speaks 18
> languages.
>
> TraceDraw collects nothing and needs no account. It is free software
> under the MIT or Apache-2.0 license; the source code, the feature
> status and the issue tracker are at github.com/igtoth/tracedraw.
>
> This is an alpha release: some features are still missing, and real
> work will turn up bugs. Reports and sample files help most.

**What's new in this version** [up to 1500 characters]: the top section
of [`CHANGELOG.md`](../CHANGELOG.md) for the version submitted.

**Product features** [up to 20, each up to 200 characters]:

1. Vector drawing: shapes, Bezier and pen curves, node editing, shaping
2. Artistic and paragraph text, text on a path, columns and drop caps
3. Gradient, pattern, texture and mesh fills; any outline and arrowhead
4. Live effects: blend, contour, shadows, envelope, extrude, lens
5. Multi-page documents with layers, master layers, guides and grids
6. Print with imposition, printers' marks and separations
7. Export PDF/X, SVG, EPS, DXF, EMF, PNG, JPEG, WebP and more
8. Opens and saves .cdr files; imports PDF, AI, SVG, EPS, DXF and PSD
9. Your own shortcuts, movable bars and exportable workspaces
10. Interface in 18 languages; free, open source, no account

**Keywords** [up to 7]: vector editor, illustration, page layout,
drawing, graphic design, print, cdr

**Copyright and trademark info**: Copyright (c) 2026 Ighor Toth and the
TraceDraw contributors. MIT or Apache-2.0.

**Screenshots** [PNG, 1366 x 768 or larger, up to 10]: the README pictures
in `docs/images/` saved as PNG (1600 x 950): the poster (`hero`), curves,
type, fills, effects, trace, export and languages.

**Store logos**: optional; the package carries its own tiles. If asked
for a square logo, `assets/icon/tracedraw-512.png` scaled to the size
requested.

## Packages

Upload `tracedraw-<version>-windows-x86_64.msix` from the release of the
version (built when the three `MSIX_` repository variables are set). The
version on the package must be higher than the last one submitted.

## Submission options: restricted capability

The package declares `runFullTrust`. Partner Center asks why; answer:

> TraceDraw is a desktop application (Win32, packaged with MSIX). It
> needs full trust to open and save the user's drawings anywhere they
> choose, to read the installed fonts, to print, and to export files to
> the folders the user picks. It collects no data and makes no network
> connections of its own; the only exception is the optional AI Drawing
> panel, which, when the user presses Draw, sends the user's request to
> the AI service the user set up with their own key.

## Notes for certification

> TraceDraw starts on its Welcome Screen; File > New makes a drawing.
> No account or sign-in is needed. Sample drawings can be made with any
> tool in the toolbox on the left.
