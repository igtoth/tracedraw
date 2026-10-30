<p align="center">
  <img src="assets/icon/tracedraw-256.png" alt="TraceDraw" width="120">
</p>

<h1 align="center">TraceDraw</h1>

<p align="center">
  <b>Vector illustration, page layout and print; an open-source, clean-room vector editor rebuilt from scratch in pure Rust.</b><br>
  Pages and layers, curves and nodes, artistic and paragraph text, fills and outlines, live effects, symbols, PDF/X and real <code>.cdr</code> files,<br>
  in one native program for Windows, macOS and Linux, and in the browser.
</p>

<p align="center">
  <img alt="Written in Rust" src="https://img.shields.io/badge/written%20in-Rust-b7410e?style=flat-square&logo=rust">
  <img alt="Windows, macOS, Linux and the browser" src="https://img.shields.io/badge/Windows%20%C2%B7%20macOS%20%C2%B7%20Linux%20%C2%B7%20Web-native-1f6fc5?style=flat-square">
  <img alt="License: MIT or Apache-2.0" src="https://img.shields.io/badge/license-MIT%20or%20Apache--2.0-444444?style=flat-square">
  <img alt="Status: alpha" src="https://img.shields.io/badge/status-alpha-d97706?style=flat-square">
  <img alt="18 interface languages" src="https://img.shields.io/badge/interface-18%20languages-7b5cff?style=flat-square">
</p>

<p align="center">
  <a href="https://github.com/igtoth/tracedraw/releases"><b>Download</b></a> &middot;
  <a href="https://igtoth.github.io/tracedraw/">Open it in the browser</a> &middot;
  <a href="docs/features.md">Feature status</a> &middot;
  <a href="CONTRIBUTING.md">Contribute</a>
</p>

<p align="center">
  <sub>Alpha: expect bugs. A <code>.cdr</code> file that does not open right? <a href="https://github.com/igtoth/tracedraw/issues/new?template=cdr_file.yml">Send it in an issue</a>: sample files are what improves the reader most.</sub>
</p>

<br>

<p align="center">
  <img src="docs/images/tracedraw-hero.jpg" alt="TraceDraw with an illustrated poster of a lake at dusk: the title is selected, the property bar shows its font, size and alignment, and the Objects panel lists the five layers and their named objects" width="100%">
  <br>
  <sub>A poster drawn on five layers, its title selected, every object named in the Objects panel.</sub>
</p>

<p align="center">
  <a href="#features">Features</a> &middot;
  <a href="#everything-in-the-box">Everything in the box</a> &middot;
  <a href="#cdr-files-read-and-written-independently">.cdr files</a> &middot;
  <a href="#made-to-be-automated">Automation</a> &middot;
  <a href="#under-the-hood">Under the hood</a> &middot;
  <a href="#get-started">Get started</a> &middot;
  <a href="#help-wanted">Help wanted</a> &middot;
  <a href="#downloads">Downloads</a>
</p>

<br>

<table>
  <tr>
    <td width="25%" valign="top">
      <h3>A full desktop workspace</h3>
      Menu bar, context-sensitive property bar, toolbox with flyouts, 41 dockable panels, colour palette, rulers, guides and a page navigator, all set out for long working sessions.
    </td>
    <td width="25%" valign="top">
      <h3>Native and light</h3>
      One program per platform built with egui, a CPU renderer for exact output and no runtime to install. No Electron, no web view; it runs on PCs from 2010 with OpenGL 2.1, and the same code runs in the browser as WebAssembly.
    </td>
    <td width="25%" valign="top">
      <h3>Opens your files</h3>
      A clean-room <code>.cdr</code> reader for format versions 1 to 27 and a writer for versions 7 to 27, plus SVG, PDF, AI, EPS, DXF, PSD, EMF, WMF, HPGL and text documents.
    </td>
    <td width="25%" valign="top">
      <h3>Ready for scripts and agents</h3>
      Every change is a command with undo, so the same engine answers the window, the command line, JavaScript and an MCP server.
    </td>
  </tr>
</table>

<br>

## Features

Every screenshot is TraceDraw itself, captured headless by
`scripts/visual/readme_shots.sh` from drawings made from scratch by
`apps/tracedraw-cli/examples/showcase.rs`. Some are in the Dark and
Medium Dark themes (Options &rsaquo; Customization &rsaquo; Appearance,
or the moon button at the right of the status bar).

<table>
  <tr>
    <td width="50%" valign="top">
      <img src="docs/images/tracedraw-curves.jpg" alt="A leaf drawn with smooth curves; the Shape tool has every node selected, showing the handles, and the property bar shows the node buttons" width="100%">
      <br>
      <sub>The Shape tool on a leaf, every node selected with its handles.</sub>
      <h3>Curves you can shape</h3>
      Draw with Freehand, Bezier, Pen, B-Spline, Polyline and 3-Point Curve, then work the nodes: cusp, smooth and symmetrical, add, delete, join, break, extract, reduce, align, line to curve and back, and an elastic mode that moves a whole run of nodes.
      <br><br>
      Shaping tools (weld, trim, intersect, simplify, front and back minus), Smear, Twirl, Attract, Repel and Roughen, Knife, Eraser, Crop and Segment Delete reshape the rest.
    </td>
    <td width="50%" valign="top">
      <img src="docs/images/tracedraw-effects.jpg" alt="A page of six live effects (Blend, Contour, Extrude, Drop Shadow, Envelope and Distort) with the blend selected and the Blend panel open on the right" width="100%">
      <br>
      <sub>Six live effects on one page; the blend's steps and accelerations in its panel.</sub>
      <h3>Effects that stay live</h3>
      Blend, Contour, Extrude, Drop Shadow, Block Shadow, Envelope, Distort, Perspective, Bevel, Symmetry, twelve Lens types (one runs any bitmap effect over what lies beneath) and Transparency with merge modes. Each one is kept with its object and stays editable from its panel or the property bar: change the steps of a contour, send a blend along a path, move an envelope's nodes.
    </td>
  </tr>
  <tr>
    <td width="50%" valign="top">
      <img src="docs/images/tracedraw-type.jpg" alt="A page of type at 100 percent: a headline, two columns of justified paragraph text with drop caps and hyphenation, and the Text panel with character and paragraph settings" width="100%">
      <br>
      <sub>Two justified columns with drop caps and hyphenation, and the Text panel.</sub>
      <h3>Type that sets properly</h3>
      Artistic text for headlines and paragraph text in frames: columns, justification, hyphenation, indents, tabs, bullets, drop caps, leading and tracking, text wrapped round objects, fit to frame and text on a path. Shaping comes from the system's fonts, OpenType features included.
      <br><br>
      Spell check, find and replace, a glyph browser and a font manager that substitutes missing fonts keep long documents in order.
    </td>
    <td width="50%" valign="top">
      <img src="docs/images/tracedraw-fills.jpg" alt="Eight tiles filled with uniform, linear, radial, conical, square, pattern, texture and mesh fills; the Interactive Fill tool shows the gradient's handles on the linear tile" width="100%">
      <br>
      <sub>Eight fill types; the Interactive Fill tool editing a gradient on the object.</sub>
      <h3>Every kind of fill</h3>
      Uniform colour in RGB, CMYK, Gray, HSB, HSL, Lab and YIQ; gradient fills with any number of stops (linear, radial, conical, square); two-colour, bitmap and vector patterns; procedural textures; mesh fills; and Area Fill for closed regions. The Interactive Fill and Mesh Fill tools edit them right on the object.
      <br><br>
      Outlines take widths, dashes, arrowheads, calligraphic nibs, caps and corners.
    </td>
  </tr>
  <tr>
    <td width="50%" valign="top">
      <img src="docs/images/tracedraw-bitmaps.jpg" alt="The poster placed as a bitmap with the Swirl dialog open, showing the bitmap before and after the effect" width="100%">
      <br>
      <sub>Effects &rsaquo; Distort &rsaquo; Swirl, previewed before and after.</sub>
      <h3>Bitmaps, too</h3>
      Place PNG, JPEG, BMP, GIF, TIFF and WebP images, or turn vectors into a bitmap, then use 98 bitmap effects in fifteen groups, each with its own dialog and a before and after preview. Effects stay in the object's list in the Properties panel, where they can be changed, hidden or removed.
      <br><br>
      Colour modes, a colour mask, background removal on this computer (no AI needed), straighten, crop and resample complete the set.
    </td>
    <td width="50%" valign="top">
      <img src="docs/images/tracedraw-trace.jpg" alt="The Trace Bitmap dialog with the Clipart preset: the bitmap before and its traced curves after, with the detail, smoothing and colour settings" width="100%">
      <br>
      <sub>Trace Bitmap with the Clipart preset: the bitmap and the curves it becomes.</sub>
      <h3>From pixels to curves</h3>
      Trace Bitmap turns images into editable curves: Quick Trace in one click, outline tracing in up to 256 colours with background removal, centreline tracing for drawings, and eight presets from technical illustration to high quality image. The dialog previews the trace before it is made.
    </td>
  </tr>
  <tr>
    <td width="50%" valign="top">
      <img src="docs/images/tracedraw-export.jpg" alt="The Export dialog set to PNG at 300 dpi: a preview of the page, a transparent background option, the size in pixels and an estimated file size" width="100%">
      <br>
      <sub>Export as PNG with a preview, the size in pixels and an estimated file size.</sub>
      <h3>Out in any format</h3>
      SVG, PDF (with PDF/X-1a, X-3 and X-4 and bleed), EPS, AI, DXF, EMF, WMF, HPGL, PSD and HTML, and PNG, JPEG, WebP, GIF, BMP and TIFF at any resolution. The Export dialog previews the result and estimates its size.
      <br><br>
      Print goes through a PDF; Print Merge fills a design from a CSV file.
    </td>
    <td width="50%" valign="top">
      <img src="docs/images/tracedraw-languages.jpg" alt="The page of type in TraceDraw with the interface in Japanese: menus, property bar, Text panel and status bar" width="100%">
      <br>
      <sub>The same page with the interface in Japanese.</sub>
      <h3>In your language</h3>
      English, Chinese (Simplified), Hindi, Spanish, French, Arabic with a right-to-left layout, Bengali, Russian, Portuguese (Brazil), Indonesian, German, Japanese, Chinese (Traditional), Italian, Dutch, Polish, Turkish and Czech, with system fonts found for every script. Options &rsaquo; Global switches the language at once.
    </td>
  </tr>
  <tr>
    <td width="50%" valign="top">
      <img src="docs/images/tracedraw-templates.jpg" alt="The New from Template dialog: a search field, source and category lists, a grid of template thumbnails with the invoice picked, and a details pane with its category and page size" width="100%">
      <br>
      <sub>File &rsaquo; New From Template with the invoice picked.</sub>
      <h3>Start from a template</h3>
      Blank pages in the usual sizes and designed templates (business cards, flyers, a poster, a letterhead, a certificate, an invoice, a banner, a social post, a label), every part an ordinary editable object. Search them, filter by category, and keep your own: File &rsaquo; Save as Template puts a drawing in the list.
    </td>
    <td width="50%" valign="top">
      <img src="docs/images/tracedraw-ai.jpg" alt="The AI Generate docker in the dark theme with a prompt, the Watercolor style and the Warm palette picked, next to the poster" width="100%">
      <br>
      <sub>The AI Generate docker, in the dark theme.</sub>
      <h3>AI with your own keys</h3>
      Create pictures from a prompt with aspect ratio, style and palette presets, remix a selected picture, create vectors (a generated illustration traced into curves), remove a picture's background, and draw shapes from a request in words. TraceDraw has no AI of its own and no account: Tools &rsaquo; Options &rsaquo; AI connects the services you choose (Google Gemini, OpenAI or compatible APIs, local servers included), and nothing is sent until you ask.
    </td>
  </tr>
  <tr>
    <td width="50%" valign="top">
      <img src="docs/images/tracedraw-painterly.jpg" alt="A landscape painted with painterly brushes (watercolour sky, ink wash and dry brush hills, bristle and fur grass, speckled flowers) with the brush picker open on the property bar" width="100%">
      <br>
      <sub>The Painterly Brush tool and its brush picker.</sub>
      <h3>Paint with brushes</h3>
      Twelve painterly brushes in three groups (chalk, charcoal, pastel; ink wash, watercolour, dry brush; bristle, fur, spray, speckle, cloud, silk ribbon) paint freehand strokes or the outline of any object. A brushstroke stays a curve: reshape it with the Shape tool, recolour it from the palette, change its brush, size or transparency, and export it to SVG and PDF.
    </td>
    <td width="50%" valign="top">
      <img src="docs/images/tracedraw-perspective.jpg" alt="Draw in Perspective in the medium dark theme: a one-point grid over a room with checked floor tiles, windows and doors on the walls and lights on the ceiling" width="100%">
      <br>
      <sub>Draw in Perspective with a one-point grid.</sub>
      <h3>Draw in perspective</h3>
      One-, two- and three-point grids with vanishing points you drag. Whatever you draw with a drawing tool lands on the floor, a wall or the ceiling in perspective, as an exact perspective effect you can still edit. Variable-width outlines, focus mode, a multipage view, an export panel for many assets at once and review comments round out the drawing window.
    </td>
  </tr>
</table>

<br>

## Everything in the box

<table>
  <tr>
    <td width="33%" valign="top">
      <h4>64 tools</h4>
      Pick &middot; Freehand Pick &middot; Free Transform &middot; Shape &middot; Smooth &middot; Smear &middot; Twirl &middot; Attract &middot; Repel &middot; Smudge &middot; Roughen &middot; Variable Outline &middot; Crop &middot; Knife &middot; Segment Delete &middot; Eraser &middot; Zoom &middot; Pan &middot; Freehand &middot; 2-Point Line &middot; Bezier &middot; Pen &middot; B-Spline &middot; Polyline &middot; 3-Point Curve &middot; Shape Recognition &middot; Sketch &middot; Brush Strokes (preset, brush, sprayer, calligraphic, expression) &middot; Painterly Brush (twelve brushes) &middot; Rectangle and 3-Point Rectangle &middot; Ellipse and 3-Point Ellipse &middot; Polygon &middot; Star &middot; Spiral &middot; Common Shapes &middot; Action Lines &middot; Graph Paper &middot; Text &middot; Table &middot; five dimension tools &middot; three connectors &middot; Drop Shadow &middot; Contour &middot; Blend &middot; Distort &middot; Envelope &middot; Extrude &middot; Block Shadow &middot; Transparency &middot; two eyedroppers &middot; Interactive Fill &middot; Area Fill &middot; Mesh Fill &middot; Outline Pen &middot; Outline Colour
    </td>
    <td width="33%" valign="top">
      <h4>41 panels</h4>
      Properties &middot; Objects &middot; Hints &middot; Transformations &middot; Undo &middot; Align and Distribute &middot; Shaping &middot; Step and Repeat &middot; Text &middot; Glyphs &middot; Colour &middot; Colour Styles &middot; Object Styles &middot; Find and Replace &middot; Scripts &middot; Palettes &middot; Lens &middot; Blend &middot; Contour &middot; Envelope &middot; Extrude &middot; Bevel &middot; Brush Strokes &middot; Bitmap Mask &middot; Object Data &middot; Links &middot; Symbols &middot; Pages &middot; Guidelines &middot; Fonts &middot; Font Sampler &middot; Corners &middot; Join Curves &middot; Coordinates &middot; Vector Mosaic &middot; Picture Mosaic &middot; Shelf &middot; Effects &middot; Export &middot; Comments &middot; AI Generate (with Draw with words)
    </td>
    <td width="33%" valign="top">
      <h4>Documents and pages</h4>
      Multi-page documents with a page sorter, layers and master layers (all, odd or even pages), guides at any angle, grids, snapping to objects, page, grid, baseline, guidelines and pixels, dynamic guides, page backgrounds, bleed, tables, symbols, clip frames and metadata. Several drawings open in tabs; auto-backup keeps unsaved work.
    </td>
  </tr>
  <tr>
    <td width="33%" valign="top">
      <h4>Colour</h4>
      Colour models from RGB and CMYK to Lab, registration colour, colour styles and harmonies, palettes and a palette manager, a document palette, ICC colour management with rendering intents, proof colours, overprint simulation and separations.
    </td>
    <td width="33%" valign="top">
      <h4>Formats</h4>
      Open: <code>.tdraw</code>, <code>.cdr</code> (versions 1 to 27) and its templates, CMX, SVG and SVGZ, PDF and AI, EPS, DXF, PSD, EMF, WMF, HPGL (PLT), TXT, RTF and DOCX, and bitmaps.
      <br><br>
      Save and export: <code>.tdraw</code> (JSON), <code>.cdr</code> (versions 7 to 27), SVG, PDF, AI, EPS, DXF, EMF, WMF, PLT, PSD, HTML, PNG, JPEG, WebP, GIF, BMP and TIFF.
    </td>
    <td width="33%" valign="top">
      <h4>The everyday essentials</h4>
      Undo and redo with a history panel &middot; duplicate, clone and step and repeat &middot; align, distribute and order &middot; object and colour styles &middot; find and replace &middot; customisable shortcuts and workspaces &middot; an interface size setting &middot; Options with every preference &middot; a splash screen and a welcome screen with recent drawings and templates.
    </td>
  </tr>
</table>

<br>

## .cdr files, read and written independently

TraceDraw's `.cdr` support lives in its own crates (`crates/cdr-format`
for the file structure, `crates/cdr` for reading and writing drawings), written
from public format notes and from files we own, never from decompiled
programs or other implementations. `CLEANROOM.md` records where each piece
of format knowledge comes from and when; `docs/cdr-format.md` records what
is confirmed and what is still assumed.

- **Containers of every generation:** the `WL` files of versions 1 and 2,
  RIFF files (versions 3 to 13, with
  compressed chunk streams from 7), ZIP packages (14 and 15) and the
  split streams of 16 and later. CMX exchange files open too.
- **What comes across:** pages, layers, master layers, guidelines,
  rectangles, ellipses, polygons and stars, curves with their node types,
  artistic and paragraph text (versions 1 to 3 and from 5; bold, italic
  and text on a path from 7), groups, clip containers, bitmaps (CMYK and cut-out), uniform,
  gradient, pattern, texture and vector pattern fills, outlines with
  arrowheads, uniform and graded transparency, embedded fonts and the
  document palette; what the reader cannot map yet is counted and
  reported, never silently dropped.
- **Writing back, in the version you need:** drawings save as `.cdr` in
  any version from 7 to 27, as well as in TraceDraw's own JSON format. A
  drawing opened from a `.cdr` saves in its own version unless you pick
  another. Save As offers every version from 7 to 27. From version 16 the file can carry the drawing's CMYK
  colour profile and the fonts its text uses (when their licence allows
  embedding).
- **Never crashes on a file:** `tracedraw-cli stress` runs mutation tests
  against the reader, opening any file always yields a document, and
  what a file may decompress to is capped.

## Made to be automated

Every menu item, tool and panel builds a command for one engine, with undo
and redo. The window, the command line, the JavaScript object model and the
MCP server all go through it, so anything a click can do, a script can do.

```sh
# Convert between any format TraceDraw opens and any it writes
tracedraw-cli convert poster.cdr poster.pdf
TRACEDRAW_DPI=300 tracedraw-cli convert poster.tdraw poster.png
TRACEDRAW_PAGE=2 tracedraw-cli convert brochure.cdr page2.png
TRACEDRAW_CDR_VERSION=15 tracedraw-cli convert logo.svg logo.cdr   # .cdr version 15 (7 to 27)

# A PDF with a bookmark per page that cannot be printed or copied
# without its permissions password
TRACEDRAW_PDF_BOOKMARKS=1 TRACEDRAW_PDF_OWNER_PASSWORD=boss \
  TRACEDRAW_PDF_DENY=print,copy tracedraw-cli convert brochure.cdr brochure.pdf

# Look inside a .cdr: the chunk tree, and what the reader made of it
tracedraw-cli inspect drawing.cdr
tracedraw-cli info drawing.cdr
TRACEDRAW_OBJECTS=1 tracedraw-cli info drawing.cdr   # every object
tracedraw-cli check saved.cdr   # what a reader would stumble on in a file TraceDraw wrote

# Serve the editor to an AI agent over the Model Context Protocol
tracedraw --mcp
# ... or let the agent edit the drawing open in the window (Options >
# Global > General > Let AI agents on this computer edit the open drawing)
tracedraw --mcp-window
```

The Scripts panel runs JavaScript against an object model of documents,
pages, layers and shapes, and records macros as scripts:

```js
// A fan of twelve rounded bars, each a little more orange.
for (let i = 0; i < 12; i++) {
  const bar = ActiveLayer.CreateRectangle(80, 120, 60, 12, 3);
  bar.Fill.ApplyUniformFill(Color.RGB(255, 60 + i * 15, 60));
  bar.Rotate(i * 15);
}
```

See [`docs/behavior/mcp.md`](docs/behavior/mcp.md) for the MCP tools.

**VBA macros** run inside the editor: open a macro project (`.gms`, or
`.bas`, `.cls` and `.frm` modules with their forms) in the Macro Manager,
run, step and edit it in the macro editor, on every platform and in the
browser. See [`docs/behavior/vba.md`](docs/behavior/vba.md).

**Draw with words** (a mode of the AI Generate docker) writes those scripts for you: describe
what to draw or change, and an AI service you set up with your own key
(the Messages API, any chat completions service, or a model server on
your own computer) writes the script, which runs as one undo step without
access to your files. TraceDraw has no AI service or account of its own;
see [`docs/behavior/ai-drawing.md`](docs/behavior/ai-drawing.md).

## Under the hood

- **Engine first:** a plain-data document model and a command engine in
  `crates/core`, with no interface code; the egui desktop app is a thin
  shell over it. The layering between crates is checked in review.
- **One renderer:** a CPU rasteriser on tiny-skia draws the canvas, the
  exports and the previews, so what is on screen is what comes out.
- **Old PCs welcome:** the window is drawn with Direct3D 12, Vulkan or
  Metal, then OpenGL, then a software adapter, whichever the computer
  can open. Cards with OpenGL 2.1 only (the Intel HD Graphics of 2010
  PCs, for example) get the fixed-function pipeline they were made for,
  and a start that comes up black or hangs restarts with the next
  renderer by itself.
- **Real text:** fonts from the system through fontdb, shaping with
  rustybuzz, a paragraph engine of its own.
- **Runs in the browser:** the whole editor compiles to WebAssembly.
- **Clean-room:** format support from public notes and owned files only;
  no proprietary code or assets.
- **Tested:** about 2,200 tests, including format round trips,
  malformed-input tests, effect geometry and interface drawing, plus
  headless visual checks that drive the real app.

## Get started

```sh
git clone https://github.com/igtoth/tracedraw
cd tracedraw
cargo run --release -p tracedraw                  # the editor
cargo run --release -p tracedraw -- drawing.cdr   # open a file
cargo test --workspace                            # the test suite
```

A recent stable Rust (1.89 or newer; CI builds with 1.97). On Linux the
editor needs X11 or Wayland, libxkbcommon and Mesa.

### What it runs on

- **Windows 10 and 11** (x64), **macOS 11 or later** (Apple silicon and
  Intel), **Linux** (x86_64 and arm64, glibc 2.35 or later: Ubuntu 22.04,
  Debian 12, Fedora 36 or newer) and any current browser. Older Linux
  distributions can use the browser version.
- **Older computers too:** a graphics card with OpenGL 2.1 is enough. An
  office PC from 2010 with Intel HD Graphics (Ironlake) under Windows 10
  runs TraceDraw with its own card since 0.19.8 (#616). Without a usable
  card, TraceDraw draws the window on the CPU.
- Minimum and recommended hardware and software, measured and
  estimated: [`docs/system-requirements.md`](docs/system-requirements.md).
- If the window still does not come up, start it with
  `tracedraw --renderer=gl` and send `tracedraw.log` (in the settings folder) in
  [an issue](https://github.com/igtoth/tracedraw/issues/new?template=bug_report.yml).

New contributors and AI agents: start with [`AGENTS.md`](AGENTS.md), then
[`CONTRIBUTING.md`](CONTRIBUTING.md) and the pages in [`docs/`](docs/).

> [!IMPORTANT]
> **Status: alpha.** Most of what a vector editor is expected to do is in
> place and covered by tests, but TraceDraw is young and has barely met
> real users: expect bugs and rough edges,
> some formats and effects that are close in look rather than identical,
> and gaps listed in [`docs/blueprint-gaps.md`](docs/blueprint-gaps.md).
> [`docs/features.md`](docs/features.md) is the honest, row-by-row picture.
> Issues with a sample file and a screenshot are the most helpful; there
> are [templates](https://github.com/igtoth/tracedraw/issues/new/choose)
> for bugs, `.cdr` files and feature requests.
>
> Issues are looked at every day, in the maintainer's spare time, so
> expect some delay; each one is answered and worked on as soon as
> possible.

## Help wanted

Anyone who wants to help is welcome, with or without writing code:

- **Try it on your own files.** A `.cdr` that does not open right, sent
  in [an issue](https://github.com/igtoth/tracedraw/issues/new?template=cdr_file.yml),
  improves the reader more than anything else (only files you may share).
  Files of versions 4 and 6, and old files that use perspective,
  pattern or PostScript fills, with a picture of how they look, are the
  most wanted: those are the parts still read from few samples.
- **Open our test files in your copy of the reference program.** The
  [`interop` branch](https://github.com/igtoth/tracedraw/tree/interop/bundles)
  holds small `.cdr` files saved by TraceDraw in versions 7 to 15; telling
  us which ones open (and which do not) is what lets TraceDraw save
  versions 7 to 13 again. Releases from 2021 on open version 15 and newer only,
  so versions 7 to 14 need CorelDRAW 2020 or older.
- **Report bugs** with the steps and a screenshot
  ([bug report](https://github.com/igtoth/tracedraw/issues/new?template=bug_report.yml)).
- **Review a translation** in your language (`apps/tracedraw/lang/`):
  native speakers catch what no test does.
- **Pick a row** of [`docs/features.md`](docs/features.md) that is not
  "works" yet, or a gap in [`docs/blueprint-gaps.md`](docs/blueprint-gaps.md),
  and send a pull request.
- **Package it** for the system you use (Flatpak, Homebrew, winget, AUR).

**A contributors team is planned:** people who help regularly (fixes,
sample files, tests against the reference program, translations) will
be invited into a team, with a ranking that shows who helped most. If
you would like to be part of it, say so in an issue.

Read [`CONTRIBUTING.md`](CONTRIBUTING.md) first: clean-room rules, the
sign-off line and how pull requests are checked.

## Downloads

Each version gets a release with installers for every platform: take the
[latest release](https://github.com/igtoth/tracedraw/releases/latest)
(what changed is in [`CHANGELOG.md`](CHANGELOG.md)). Every push to `main`
also refreshes the **Nightly build** pre-release. `<version>` in the file
names is the version number.

| Platform | Files |
|---|---|
| Windows (x64) | `tracedraw-<version>-windows-x86_64-setup.exe` (installer), `tracedraw-<version>-windows-x86_64-portable.zip` (keeps its settings in the `tracedraw-settings` folder beside the program; not signed yet: if SmartScreen warns, More info > Run anyway) |
| macOS (Apple silicon, Intel) | `tracedraw-<version>-macos-arm64.dmg`, `tracedraw-<version>-macos-x86_64.dmg` (unsigned for now: right-click, Open) |
| Linux (x86_64, arm64) | `.deb` (`_amd64` or `_arm64`), `tracedraw-<version>-linux-amd64.AppImage` or `-linux-arm64.AppImage`, `tracedraw-<version>-linux-x86_64.tar.gz` or `-linux-arm64.tar.gz` |
| Web | no download: [open it in the browser](https://igtoth.github.io/tracedraw/), nothing to install |
| Android (64-bit ARM; x86_64 Chromebooks) | `tracedraw-<version>-android-arm64-v8a-debug.apk` (`-x86_64-` for Chromebooks) in the releases and the nightly, signed with a debug key (remove the installed one before installing a build of another run); `scripts/android/build.sh` builds debug-signed APKs and an App Bundle for Google Play, and CI keeps them with each run as the `android-apk` and `android-aab` artifacts after starting the app on Android emulators (see [`docs/behavior/android.md`](docs/behavior/android.md) and [`docs/android-play.md`](docs/android-play.md)) |
| Web | no download: [open it in the browser](https://igtoth.github.io/tracedraw/), nothing to install; on an Android tablet, an iPad or a desktop the browser can also install the page as an app that opens full screen and starts offline (see [`docs/behavior/web.md`](docs/behavior/web.md), Installing on a tablet) |

Each download includes `tracedraw-cli` next to the editor. Packaging sources
are in [`packaging/`](packaging/); the browser build is described in
[`docs/behavior/web.md`](docs/behavior/web.md).

## Code signing policy

Windows releases are built from this repository by GitHub Actions
([`build.yml`](.github/workflows/build.yml)) and nowhere else. The
downloads are not code-signed yet: if SmartScreen warns, More info > Run
anyway. The Microsoft Store version, once published, is signed by the
Store. When a certificate is set up, a signing request is made only for a
version tag, and only after an approver accepts it; how the build signs
is described in [`docs/code-signing.md`](docs/code-signing.md).

| Role | Members |
|---|---|
| Committers and reviewers | [Ighor Toth](https://github.com/igtoth) |
| Approvers | [Ighor Toth](https://github.com/igtoth) |

Privacy policy: this program will not transfer any information to other
networked systems unless specifically requested by the user or the person
installing or operating it. TraceDraw opens web pages (help, the project's
pages) in your browser only when you choose to. The one automatic
connection is the update check: the first time TraceDraw starts it asks
whether to check for updates (Always check / Never check) and connects to
nothing before you answer; with Always check it asks GitHub
(`api.github.com`) once a day whether a newer release is out, sending no
data about you or your drawings. Turn it off at any time in Tools >
Options > Global > General, "Check for updates at start". The full policy is in
[`PRIVACY.md`](PRIVACY.md).

## Documentation

| Document | What it covers |
|---|---|
| [User Help](https://igtoth.github.io/tracedraw/help/en/index.html) | the manual, with screenshots; every "?" button and F1 in the app open its page in the interface language |
| [`docs/features.md`](docs/features.md) | every capability and its status |
| [`docs/behavior/`](docs/behavior/) | defaults, shortcuts and formulas, tool by tool |
| [`docs/architecture.md`](docs/architecture.md) | crates, data flow, units, rendering, text |
| [`docs/decisions.md`](docs/decisions.md) | the choices behind the stack |
| [`docs/acceptance.md`](docs/acceptance.md) | measurable acceptance criteria |
| [`docs/cdr-format.md`](docs/cdr-format.md) | what the `.cdr` reader assumes and what is confirmed |
| [`docs/i18n.md`](docs/i18n.md) | interface languages |
| [`docs/roadmap.md`](docs/roadmap.md) | milestones |
| [`docs/code-signing.md`](docs/code-signing.md) | signing the Windows downloads and the Microsoft Store package |
| [`CLEANROOM.md`](CLEANROOM.md) | where the knowledge of every file format comes from |
| [`ATTRIBUTION.md`](ATTRIBUTION.md) | bundled assets and their licenses |
| [`CONTRIBUTORS.md`](CONTRIBUTORS.md) | the people who contributed code, tests and reports |

---

## License and credits

TraceDraw is dual-licensed under [MIT](LICENSE-MIT) or
[Apache-2.0](LICENSE-APACHE), at your option.
Copyright &copy; 2026 Ighor Toth and the TraceDraw contributors.

Written by Ighor Toth with Claude Code, Anthropic's AI coding assistant,
as co-author: commits it helped write carry a `Co-Authored-By` line, and
the About box says so too.

Code contributed by others is listed, person by person, in
[CONTRIBUTORS.md](CONTRIBUTORS.md).

Bundled fonts, icons and other assets keep their own open licenses; each
is listed with its author, source and license in
[ATTRIBUTION.md](ATTRIBUTION.md). Every drawing in the screenshots was made
for TraceDraw.

<sub>TraceDraw is an independent open-source project, not affiliated with,
sponsored by or endorsed by any other software vendor. Trademarks and
product names belong to their owners; file format names such as
<code>.cdr</code>, <code>.ai</code>, <code>.psd</code> and <code>.pdf</code>
are used only to describe the files TraceDraw reads and writes. The
software is provided "as is", without warranty of any kind, as the
licenses state.</sub>
