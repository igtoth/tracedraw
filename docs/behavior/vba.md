# VBA macros

## Object model

VBA macros are compiled to JavaScript and run against the object model
VBA macros for drawing suites use (`scripting::run_vgcore`,
`apps/tracedraw/src/scripting/vg_model.js` over the `vg_*` host methods
in `scripting/vg.rs`). Member names, argument names and order, 1-based
indexes and the `cdr*` constants are the ones those macros are written
against, so the compiled code calls them unchanged. TraceDraw's own
JavaScript object model (the Scripts panel, lengths in millimetres) is
separate and unchanged.

Rules that hold everywhere:

- **Units.** Every length (positions, sizes, outline widths, radii,
  page sizes) is in `ActiveDocument.Unit`: inches unless the macro sets
  it (`cdrInch`, `cdrMillimeter`, `cdrCentimeter`, `cdrPoint`, `cdrPica`,
  `cdrPixel` = 1/96 in, `cdrFoot`, `cdrYard`, `cdrMeter`, `cdrKilometer`,
  `cdrMile`, `cdrDidots`, `cdrCicero`, `cdrAgate`, `cdrTenthMicron`).
  The unit holds for the rest of the run. Coordinates start at the
  page's lower left corner, y up; angles are degrees, counter-clockwise.
- **Reference point.** `ActiveDocument.ReferencePoint` (`cdrCenter`
  unless set; `cdrTopLeft`, `cdrBottomLeft`...) is the point
  `PositionX`/`PositionY` read, `SetPosition` places and `SetSize` and
  `Stretch` scale about.
- **One step.** Everything a run changes is one undo step;
  `BeginCommandGroup`/`EndCommandGroup` are accepted and change nothing.
  A failure keeps the changes made before it.
- **Out parameters.** Methods that hand values back through ByRef
  arguments return them as an array in parameter order: `GetPosition`
  gives `[x, y]`, `GetSize` `[w, h]`, `GetBoundingBox` `[x, y, w, h]`.
- **Named arguments.** Methods are declared with their reference
  parameter names (`Left`, `Top`, `Right`, `Bottom`, `CornerUL`...), so
  named arguments match by name.
- **Collections** are 1-based, take an index or a name in `Item`, have
  `Count`, and iterate (`For Each`) in drawing order.

### Application (also global)

`ActiveDocument`, `ActivePage`, `ActiveLayer`, `ActiveSelection` (the
selection as one object, type `cdrSelectionShape`, with `Shapes`),
`ActiveSelectionRange`, `ActiveShape` (the first selected object),
`ActiveWindow` (`Refresh`, `ActiveView`, accepted without effect),
`Documents` (the active drawing only), `Version` "27.0", `VersionMajor`
27, `VersionMinor`, `Name` "TraceDraw", `Visible`, `Optimization`,
`EventsEnabled`, `Refresh`, `CreateColor(ColorString)` ("#RRGGBB",
"RGB255,USER,r,g,b", "CMYK100,USER,c,m,y,k", "GRAY255,USER,v"),
`CreateRGBColor`, `CreateCMYKColor`, `CreateGrayColor`,
`CreateHSBColor`, `CreateShapeRange`, `CreateCurve`. `CreateDocument`
and `OpenDocument` raise an error: a macro works on the active drawing.
`Status` (`BeginProgress`, `Progress`, `EndProgress`, `SetMainText`:
the text and percentage go to the status line), `ColorManager`
(`DefaultColorContext` profile names, `ColorEngine`), `CorelScript`
(`RedrawScreen`, no effect), `UILanguage`, `CreateStructCreateOptions`.

### Document

`Unit`, `ReferencePoint`, `Pages`, `PageCount`, `ActivePage`,
`ActiveLayer`, `ActiveShape`, `Selection()`, `SelectionRange()`,
`SelectableShapes`, `ClearSelection`, `SelectAll`, `BeginCommandGroup`,
`EndCommandGroup`, `AddPages(n)`, `AddPagesEx(n, Width, Height)`,
`InsertPages(n, BeforePage, Page)`, `CreateCurve`, `Save` (to its file),
`SaveAs(FileName)` (`.cdr`, else the native format), `Export(FileName,
Filter)`, `ExportEx` and `ExportBitmap(FileName, Filter, Range,
ImageType, Width, Height, ResolutionX)` (both return a filter whose
`Finish` writes the file), `PublishToPDF`, `Name`, `Title`, `FileName`,
`FilePath`, `FullFileName`, `Dirty`, `Resolution`. Export writes the
active page: SVG, PDF (all pages), EPS, CDR, the native format, PNG, JPEG,
BMP, TIFF, GIF and WebP, chosen by the `cdr*` filter constant or by the
file's extension; bitmaps at the resolution given, or scaled to
`Width` x `Height` pixels, else 96 dpi. `Undo` and `Redo` raise an error
(the run is one step).

### Page

`Shapes` (the objects on its layers), `Layers`, `AllLayers` (with the
master layers), `ActiveLayer`, `SelectableShapes` (on visible, unlocked
layers), `Name` (read and write), `Index`, `SizeWidth`, `SizeHeight`,
`SetSize`, `GetSize`, `Orientation` (`cdrPortrait`, `cdrLandscape`;
setting it swaps the sides), `LeftX`, `RightX`, `TopY`, `BottomY`,
`CenterX`, `CenterY`, `Activate`, `Delete`, `CreateLayer`, `FindShapes`,
`FindShape`.

### Layer

`Shapes`, `Name`, `Visible`, `Printable`, `Editable`, `Master`,
`IsSpecialLayer`, `Page`, `Index`, `Activate`, `Delete`, and the
creation methods, which give new objects no fill and the default
outline:

- `CreateRectangle(Left, Top, Right, Bottom, CornerUL, CornerUR,
  CornerLR, CornerLL)`: corners as roundness, percent of half the
  shorter side.
- `CreateRectangle2(x, y, Width, Height, RadiusUL, RadiusUR, RadiusLR,
  RadiusLL)`: radii as lengths.
- `CreateEllipse(Left, Top, Right, Bottom, StartAngle, EndAngle, Pie)`,
  `CreateEllipse2(CenterX, CenterY, Radius1, Radius2, StartAngle,
  EndAngle, Pie)`.
- `CreatePolygon(Left, Top, Right, Bottom, Sides, SubPaths, Complexity,
  Star, StarComplexity, Sharpness)`, `CreatePolygon2(CenterX, CenterY,
  Radius, Sides)`.
- `CreateCurve(Curve)`: a curve built with `CreateCurve`,
  `CreateSubPath(x, y)`, `AppendLineSegment(x, y)`,
  `AppendCurveSegment2(x, y, StartingControlPointX,
  StartingControlPointY, EndingControlPointX, EndingControlPointY)` and
  `Closed`.
- `CreateLineSegment`, `CreateCurveSegment(StartX, StartY, EndX, EndY,
  StartingControlPointLength, StartingControlPointAngle,
  EndingControlPointLength, EndingControlPointAngle)`.
- `CreateArtisticText(Left, Bottom, Text, LanguageID, CharSet, Font,
  Size, Bold, Italic, Underline, Alignment)`,
  `CreateParagraphText(Left, Top, Right, Bottom, Text, ...)`.
- `Import(FileName)` (pictures).

### Shapes, ShapeRange and the selection

`Shapes` (of a layer, page or group): `Count`, `Item`, `All()`,
`Range(IndexArray)`, `AllExcluding`, `FindShapes(Name, Type, Recursive,
Query)`, `FindShape(Name, Type, StaticID, Recursive, Query)`.

`ShapeRange` (creatable with `New`): `Count`, `Item`, `Shapes`,
`FirstShape`, `LastShape`, `Add`, `AddRange`, `Remove`, `RemoveAll`,
`RemoveRange`, `IndexOf`, `Exists`, `Range`, `ReverseRange`, `Reverse`,
`Sort("@shape1.Left < @shape2.Left")` (Left, Right, Top, Bottom,
CenterX, CenterY, Width, Height, Name; `>` sorts down), `Group`,
`Combine`, `Ungroup`, `UngroupAll`, `BreakApart`, `Duplicate`, `Clone`,
`FindShapes`, `FindShape`, `AlignAndDistribute(MethodH, MethodV,
AlignTo, DistributeArea, ..., PointX, PointY, DistributeRect)` (align
to the range's last shape, the page edge or centre, or a point; spread
edges or centres, or make the gaps equal, across the range, the page or
a rectangle; the grid raises an error), `DistributeHorizontally`, `DistributeVertically`, and every geometry and
style member below, acting on the range as one.

Queries take comparisons of `@name`, `@type` (rectangle, ellipse,
curve, polygon, bitmap, text, group, symbol) and `@fill.type` (none,
uniform, fountain, pattern, texture) with `=` or `<>`, joined by `and`
and `or`: `"@type = 'rectangle' and @name <> 'Frame'"`.

### Shape

Shared with ranges and the selection:

- **Position and size:** `PositionX`, `PositionY`, `SizeWidth`,
  `SizeHeight`, `LeftX`, `RightX`, `TopY`, `BottomY`, `CenterX`,
  `CenterY` (read and write), `GetPosition`, `GetSize`,
  `GetBoundingBox`, `SetPosition`, `SetSize` (a zero or missing side
  keeps the proportions), `SetSizeEx(CenterX, CenterY, Width, Height)`,
  `SetBoundingBox(x, y, Width, Height, KeepAspect)`.
- **Transforms:** `Move`, `Rotate` (about the rotation centre),
  `RotateEx`, `Skew`, `SkewEx`, `Stretch`, `StretchEx`, `Flip(cdrFlip...)`.
- **Style:** `Fill`, `Outline`, `Transparency`, `ApplyUniformFill`,
  `ApplyNoFill`, `SetOutlineProperties`.
- **Edit:** `Delete`, `ConvertToCurves`.
- **Selection:** `AddToSelection`, `RemoveFromSelection`,
  `CreateSelection`.
- **Order:** `OrderToFront`, `OrderToBack`, `OrderForwardOne`,
  `OrderBackOne`, `OrderFrontOf`, `OrderBackOf`.
- **Alignment:** `AlignToShape(cdrAlign..., Shape)`, `AlignToPage`,
  `AlignToPageCenter`, `AlignToPoint`.
- **State:** `Locked`, `Visible`, `OverprintFill`, `OverprintOutline`.

`Shape` alone:

- **Identity:** `Type` (`cdrRectangleShape`, `cdrEllipseShape`,
  `cdrCurveShape`, `cdrPolygonShape`, `cdrBitmapShape`, `cdrTextShape`,
  `cdrGroupShape`, `cdrSymbolShape`; a clip frame reads as its frame's
  type), `Name`, `StaticID`, `IsValid`, `IsSame`, `Selected`.
- **Rotation:** `RotationAngle`, `RotationCenterX`, `RotationCenterY`
  (kept for the run).
- **Copies:** `Duplicate(OffsetX, OffsetY)` (with new ids inside groups
  too), `Clone`.
- **Shaping:** `BreakApart`, `BreakApartEx`, `Weld`, `Trim` and
  `Intersect(TargetShape, LeaveSource, LeaveTarget)` (they return the
  new curve).
- **Groups and frames:** `Ungroup`, `UngroupEx`, `UngroupAll`,
  `UngroupAllEx`, `Shapes` (a group's), `Layer`, `Page`, `ParentGroup`,
  `ClipFrame` (`Shapes` inside a clip frame), `ClipFrameParent`.
- **Views of the object:**
  - `Curve`: `Nodes`, `SubPaths`, `Segments`, `Closed`, `Length`.
    Nodes have `PositionX`, `PositionY`, `SetPosition`, `Move`, `Type`
    (`cdrCuspNode`, `cdrSmoothNode`, `cdrSymmetricalNode`), `IsEnding`,
    `SubPathIndex`, `Index` and `Delete`; subpaths have `Nodes`,
    `Closed`, `StartNode` and `EndNode`; segments have `StartNode` and
    `EndNode`.
  - `Text`: `Type`, `Replace(OldText, NewText, CaseSensitive)` and
    `Story`, with `Text`, `Font`, `Size` in points, `Bold`, `Italic`,
    `Underline`, `Alignment` (`cdrLeftAlignment`...), `Length`,
    `Characters.Count` and `Words.Count`.
  - `Rectangle`: `CornerUpperLeft`... as roundness, `RadiusUpperLeft`...
    as lengths, `SetRoundness`, `SetRadius`, `Width`, `Height`.
  - `Ellipse`: `StartAngle`, `EndAngle`, `Type`, `Pie`, `IsCircle`.
  - `Polygon`: `Sides`, `Type`, `Sharpness`.
  - `Bitmap`: `SizeWidth` and `SizeHeight` in pixels.
- **Data:** `Effects` (read only: `Count`, `Item(i).Type`), `ObjectData`
  and `Properties` (`Item(name).Value` read and write, `Get`, `Set`).

### Fill, Outline, Transparency

- **`Fill`:** `Type` (`cdrNoFill`, `cdrUniformFill`, `cdrFountainFill`,
  `cdrPatternFill`, `cdrTextureFill`; a mesh reads as uniform),
  `UniformColor` (a colour tied to the object: `RGBAssign` and the other
  assignments change the fill), `ApplyNoFill`, `ApplyUniformFill(Color)`,
  `ApplyFountainFill(StartColor, EndColor, Type, Angle)` (linear), and
  `CopyAssign`.
- **`Outline`:** `Type` (`cdrNoOutline`, `cdrOutline`), `Width`, `Color`
  (tied to the object like the fill's), `Style` (`DashCount`,
  `SetDashes`), `LineCaps`, `LineJoin`, `SetNoOutline`,
  `SetProperties(Width, Style, Color)`, and `CopyAssign`.
- **`Transparency`:** `Uniform`, `ApplyUniformTransparency(Value)` (0 to
  100), `ApplyNoTransparency`.

### Color

Creatable with `New`.

- **Assign:** `RGBAssign`, `CMYKAssign`, `CMYAssign`, `GrayAssign`
  (0 to 255), `BWAssign`, `HSBAssign`, `HLSAssign`, `LabAssign`,
  `RegistrationAssign`, `CopyAssign`, `StringAssign`.
- **Convert:** `ConvertToRGB`, `ConvertToCMYK`, `ConvertToGray`,
  `ConvertToHSB`, `ConvertTo(cdrColor...)`.
- **Read:** `Type` (`cdrColorRGB`, `cdrColorCMYK`, `cdrColorGray`,
  `cdrColorHSB`, `cdrColorHLS`, `cdrColorLab`, `cdrColorSpot`,
  `cdrColorRegistration`), `RGBRed`, `RGBGreen`, `RGBBlue`, `RGBValue`,
  `CMYKCyan`, `CMYKMagenta`, `CMYKYellow`, `CMYKBlack` (all read and
  write), `Gray`, `HSBHue`, `HSBSaturation`, `HSBBrightness`,
  `HexValue`, `Name`, `IsSpot`, `IsCMYK`, `IsSame`.

### Constants

The enumerations macros use are globals with their numeric values.

Checked member by member against the published developer reference
(enumeration pages listing each member's value):
`cdrUnit` (including `cdrUnitQ` and `cdrUnitH`, both 0.25 mm),
`cdrReferencePoint`, `cdrShapeType`, `cdrFillType`, `cdrFilter` (every
member, `cdrWEBP` from a later version of the reference), `cdrImageType`,
`cdrEffectType`, `cdrAlignment`, `cdrFontLine`, `cdrFontCase`,
`cdrSegmentType`, `cdrOutlineType`, `cdrOutlineLineCaps`, `cdrMergeMode`,
`cdrTransparencyAppliedTo`, `cdrDistributeArea`, `cdrAlignDistributeH`,
`cdrAlignShapesTo`, and the members TraceDraw defines of
`cdrTextLanguage` and `cdrCursorShape`.

Checked against published type definitions rather than the reference:
`cdrColorType`, `cdrTextType` and `cdrTriState`.

Not yet checked, with the values macros commonly assume:
`cdrFountainFillType`, `cdrOutlineLineJoin`, `cdrFlipAxes`,
`cdrAlignType` (`cdrAlignLeft` 1, `cdrAlignRight` 2, `cdrAlignHCenter`
3, `cdrAlignTop` 4, `cdrAlignBottom` 8, `cdrAlignVCenter` 12),
`cdrAlignDistributeV` (taken to mirror `cdrAlignDistributeH`: 1 top, 2
bottom, 3 centre, 4 to 6 spread those lines, 7 equal gaps),
`cdrNodeType`, `cdrPageOrientation`, `cdrExportRange`,
`cdrTransparencyType`, `cdrLensType`, `cdrViewType`,
`cdrAntiAliasingType`, `cdrImportMode`, `cdrFileVersion`,
`cdrCompressionType` and `cdrShapeLinkType`.

Method parameters carry the reference names, so named arguments match;
checked for `CreateRectangle`, `CreateRectangle2`, `CreateEllipse2`,
`SetPosition`, `SetSize`, `SetSizeEx`, `Move`, `Rotate`, `Duplicate`,
`ApplyUniformFill`, `ApplyFountainFill`, `RGBAssign`, `CMYKAssign`,
`CopyAssign`, `ConvertToBitmapEx`, `Resample`, `FindShapes`, `Export`,
`ExportEx` and `ExportBitmap`. Trailing parameters TraceDraw has no use
for are accepted and ignored.

### Coverage from published macros

Four published macro projects (60 modules) were read only to count
which members they call; none of their code is in TraceDraw. The members
called three times or more, with what they do here (W works; N works with
no visible effect, because the feature it controls is a window mode or
does not exist; E raises an error that names the member; F belongs to
macro forms, not to this object model; R belongs to the VBA runtime or to
other libraries):

| Member | Uses | Here |
|---|---|---|
| Count, Item, Add, AddRange, RemoveAll, RemoveRange, Range, ReverseRange, All, Shapes, Layers, Pages | 195, 13, 99, 5, 66, 7, 5, 64, 53, 60, 19, 15 | W |
| Type, Name, Layer, Page, Delete, Duplicate, Activate | 173, 96, 43, 39, 7, 3, 16 | W |
| Fill, UniformColor, Fountain, Colors, ApplyUniformFill, CopyAssign | 105, 56, 22, 19, 3, 13 | W (fountain `Steps` set: E) |
| Outline, Width, Color, ScaleWithShape | 103, 31, 109, 10 | W (`MiterLimit` set: E) |
| CMYKCyan, CMYKMagenta, CMYKYellow, CMYKBlack, ConvertToRGB, ConvertToCMYK, ConvertToGray, Tint | 25 each, 10, 7, 3, 4 | W (`Tint` set on a non-spot colour: E) |
| Text, Story, Words, Range, Size, Bold, Italic, Font, LineSpacing, Overflow, IsArtisticText | 100, 12, 3, 5, 9, 9, 9, 3, 8, 4, 3 | W (`LanguageID`: E) |
| Bitmap, ResolutionX, ResolutionY, Mode, ConvertTo, Resample, ExternallyLinked, Resolution | 51, 12, 12, 7, 14, 5, 3, 4 | W |
| ConvertToBitmapEx, ConvertToCurves | 6, 4 | W |
| ClipFrame, ClipFrameParent | 44, 8 | W |
| EnterEditMode, LeaveEditMode | 3, 3 | N |
| Effects, DropShadow, Separate, Clear | 14, 6, 7, 14 | W (a separated shadow loses its blur) |
| MergeMode | 6 | W for normal; others E |
| Transparency | 14 | W (uniform; other kinds read their type) |
| Curve, Nodes, SubPaths, FirstNode, LastNode, Selection, Area, Length | 11, 4, 6, 22, 4, 9, 7, 3 | W |
| CanHaveFill, CanHaveOutline | 19, 21 | W |
| Visible, Printable, Editable, IsSpecialLayer, Master, MasterPage, UnlockAllShapes | 14, 14, 14, 12, 7, 4, 6 | W |
| IsGuidesLayer, IsGridLayer | 6, 6 | W (always false: guides and grid are not layers) |
| CreateGuide, Guide | 7, 3 | W |
| PositionX, PositionY, SizeWidth, SizeHeight, Height, SetSize, GetSize, SetPosition, GetPosition, Rotate | 16, 16, 9, 9, 5, 7, 6, 5, 3, 4 | W |
| Unit, ReferencePoint, Units | 18, 13, 3 | W |
| ClearSelection, CreateSelection, AddToSelection, FindShapes, OrderBackOf, CreateRectangle2, ActiveLayer | 30, 8, 7, 6, 3, 3, 3 | W |
| OverprintFill, OverprintOutline | 6, 5 | W |
| Status, BeginProgress, Progress, EndProgress | 22, 9, 14, 9 | W (shown in the status line when the run ends) |
| BeginCommandGroup, EndCommandGroup, PreserveSelection, SaveSettings, RestoreSettings | 4, 4, 6, 3, 3 | W |
| ColorContext, DefaultColorContext, ColorEngine | 6, 3, 4 | W (profile names in use) |
| ActiveWindow, ActiveView, OriginX, OriginY | 4, 13, 3, 3 | W (the window's centre); view changes N |
| CorelScript, RedrawScreen | 5, 4 | N |
| Finish | 3 | W (exports and imports) |
| Paste, FrameWork, Close | 3, 3, 3 | E |
| PageWidth, PageHeight | 3, 3 | E (the options of a new document: `CreateDocumentEx` is E) |
| AddItem, SpecialEffect, Enabled, Caption, Show, SelText, ListIndex, List, ListCount, ControlTipText, SelLength, Value | | F |
| Number, Description, Kill, Date, Case, ToString | | R |

The constants they use (128 names) all exist, except five names the
projects define themselves and six enumeration names used only to
declare variables (`cdrShapeType`, `cdrColorType`, `cdrImageType`,
`cdrUnit`, `cdrTextType`, `cdrAntiAliasingType`).

The remaining rarer members follow the same rules:

- Works: `MoveToLayer`, `OrderIsInFrontOf`, `AddToClipFrame`,
  `FindAnyOfType`, `SelectShapesAtPoint`, `ImportEx`, `DrawingOriginX`
  and `DrawingOriginY` (0: the origin is the page corner).
- Raise a named error: `GetUserClick`, `GetUserArea`, `DesktopLayer`,
  `UserAssignEx`, `PasteSpecial`, `StartShape`, `Fill.Pattern`,
  `GetPixelColor`, `Quit`, `VBE`.

### Not supported

- More than one drawing: `CreateDocument`, `OpenDocument`, and
  `Documents` beyond the active drawing.
- `Undo` and `Redo` inside a run, `Paste` and `PasteSpecial`.
- Window and interactive members: `GetUserClick`, `GetUserArea`,
  `FrameWork`, `VBE`, `Quit`, and the colour picker (`UserAssignEx`).
- Arrowheads: `StartArrow` and `EndArrow` read as nothing and setting
  them is ignored.
- The text's own language (`Story.LanguageID`), texture resolutions,
  fountain steps, a miter limit other than the renderer's, shadow merge
  modes other than normal.
- Fountain fills other than linear.
- Hatch and PostScript fills.
- Editing a curve built by a macro after it became an object
  (`AppendCurve`, `CreateNodes`).
- Segment types.
- Queries beyond `@name`, `@type` and `@fill.type`.
- Events.
- Export of the selection or of a range of pages: export writes the
  active page, and PDF writes every page.
- The view: `ActiveWindow.ActiveView` takes calls without effect.

### Checks (`scripting::vg::tests`)

- Units, both ways:
  - `cdrMillimeter` and `cdrInch` lengths land in the drawing exactly;
  - a 2 x 1 inch rectangle reads back as 2 x 1.
- Reference points: `SetPosition(0, 0)` with `cdrBottomLeft` puts the
  lower left corner on the page origin.
- Collections:
  - `Item` is 1-based and takes names;
  - `For Each` sees every object;
  - `FindShapes` with `Recursive` finds objects inside groups;
  - a group's `Shapes` lists its children.
- Colours and outlines:
  - `Fill.UniformColor.RGBAssign` changes the fill;
  - CMYK components read back;
  - the outline width follows the unit.
- Text: the story's text, size, bold and alignment change the text
  object.
- Shaping: `Weld` replaces both objects with one curve; `Group` and
  `UngroupAll` keep every id unique, also when pages are added in the
  same run.
- Community-style macros:
  - resize the selection to 20 x 10 mm;
  - number every object with a text at its centre;
  - lay out a 3 x 3 grid;
  - replace a colour across the page, inside groups too;
  - export every page as PNG.
- Members published macros use (one test per area):
  - progress in the status line;
  - bitmap resolution, mode, resampling and conversion;
  - converting objects to bitmaps;
  - clip frame contents and parents;
  - effects with drop shadows, their merge mode and separating them;
  - guides;
  - fountain colours and transparency;
  - text words, ranges, overflow and line spacing;
  - curve areas and the Shape tool's node selection;
  - document settings, master page, layers, unlocking, view origin and
    colour profiles;
  - the named errors;
  - the constants they use.
## Language

The `tracedraw-vba` crate compiles VBA modules (`.bas`, `.cls`, `.frm`
code) to JavaScript that runs on the same engine as TraceDraw scripts,
over the same object model. This page covers the language; the object
model is documented with scripting.

### API

- `compile(&[Module]) -> Result<Program, CompileError>`: all modules of
  a project are compiled together so calls between modules resolve.
  `compile_with(&modules, &Options { host_names, form_controls })` also
  takes the host's global names and each form's control names, which
  are left to run time under `Option Explicit` instead of failing.
  `Module { name, kind: Standard | Class | Form | Document, source }`.
  A `CompileError` names the module and the VBA line.
- `RUNTIME_JS`: the VBA runtime (`__v`), evaluated once before the
  program.
- `entry_call(&program, module, proc)`: the JavaScript that runs one
  procedure. Run-time errors come back as
  `Run-time error 'n': description (Module.Proc, line l)`; `End` stops
  quietly.
- `list_macros(&modules)`: the public parameterless Subs of standard
  and document modules (not `Option Private Module`), the list a
  Macros dialog shows.
- `Program::vba_line(js_line)` maps a JavaScript line back to the
  module and VBA line.

### Host hooks

| Name | Use |
|---|---|
| `__host_msgbox(prompt, buttons, title)` | MsgBox; returns the button value (`vbOK` = 1 when missing) |
| `__host_inputbox(prompt, title, default)` | InputBox; returns the text |
| `__host_names` | host globals declared with `const`/`class`, so lowercase names still resolve |
| `fn.__params = ["x", ...]` | parameter names of a host method, needed for named arguments |
| `__host_form_create/show/hide/unload(form)` | forms; the host fills `form.__controls` |
| `form.__event(control, event, args)` | the host fires `Control_Event` handlers |
| `__host_setting(op, app, section, key, value)` | GetSetting, SaveSetting, DeleteSetting, GetAllSettings; without it settings last for the run |

### Supported

Case-insensitive names everywhere, type suffixes, `&H`/`&O`, date
literals, line continuation, `#If`/`#Const` (VBA7 and Win64 true),
`Option Explicit/Base/Compare/Private Module`, Dim/Static/Const/ReDim
Preserve/Erase, arrays with any bounds and up to many dimensions, UDTs
(value copies), Enums, Sub/Function/Property Get/Let/Set, ByRef (default)
and ByVal, Optional with defaults, IsMissing, ParamArray, named
arguments, If (block and single line), Select Case (`Is`, `To`, lists),
For/Next, For Each, Do/Loop in all forms, While/Wend, With (nested),
Exit, GoTo to labels and line numbers, On Error Resume Next / GoTo
label / GoTo 0, Resume, Resume Next, Resume label, Err object (Number,
Description, Source, Raise, Clear), End, Set and object identity (`Is`,
`Nothing`), TypeOf ... Is, `!` access, default members (host `Item`,
class `VB_UserMemId = 0`), class modules with properties, Me,
Class_Initialize, forms code-behind (UserForm_Initialize, control
events, Show/Hide/Unload, default instances), Debug.Print with `;` and
`,` zones, Mid statement, Variant operators with VBA coercion, Like,
banker's rounding for integer conversions.

Built-ins: string functions (Left, Mid, InStr, InStrRev, Replace,
Split, Join, Trim, StrComp, StrConv, Format, Space, String, Chr, Asc,
Val, Hex, Oct...), math (Abs, Int, Fix, Round, Sqr, Rnd with the VBA
generator, Randomize...), conversions (CInt, CLng, CDbl, CStr, CBool,
CDate, CCur, CVar...), tests (IsNumeric, IsDate, IsEmpty, IsNull,
IsArray, IsObject, VarType, TypeName), arrays (Array, LBound, UBound,
Filter), dates (Now, Date, Time, DateSerial, DateAdd, DateDiff,
DatePart, Year...Second, Weekday, MonthName, Timer), MsgBox, InputBox,
IIf, Choose, Switch, GetSetting, SaveSetting, DeleteSetting,
GetAllSettings, Collection, `CreateObject("Scripting.Dictionary")`
and the vb constants.

### Not supported

These compile and fail at run time with error 445
`Not supported in TraceDraw: ...` (or 429 for objects):

- `Declare` and any Windows API call.
- File statements: Open, Close, Print #, Write #, Input #, Line Input,
  Get, Put, Kill, Name, MkDir, RmDir, ChDir, FileCopy, Dir.
- COM objects other than `Scripting.Dictionary` (FileSystemObject,
  Excel, Word, Shell...).
- GoSub/Return, On x GoTo/GoSub, AddressOf, Implements.
- RaiseEvent does nothing; WithEvents is ignored; Class_Terminate is
  not called.
- GoTo reaches labels in the same block, in the blocks around it and
  in the other branches of the same If; not labels inside a loop or a
  With from outside it.
- Named arguments to host methods need `__params`, otherwise error 448.
- Debug.Print does not add the leading space VBA prints before
  positive numbers; dates format as m/d/yyyy.

## Checks

- Given `For Each s In ActiveSelection.Shapes`, when it runs, then each
  selected shape is visited once (test `walks_the_selection...`).
- Given `On Error GoTo H` and a handler that fixes the divisor and calls
  `Resume`, then the failed statement runs again and succeeds.
- Given `Open "x" For Output As #1`, then the run stops with error 445
  naming the Open statement.
- Given `Tool.Place(label:="L", y:=2, x:=1)` with `__params`, then the
  host receives `(1, 2, "L")`.

## Macro files and forms

The `tracedraw-vba-project` crate (`crates/vba-project`) reads the files
VBA macros come in, so their code and forms can run in TraceDraw. It
follows the public MS-OVBA and MS-OFORMS specifications.

### Project files

- A compound file holding a VBA project (`.gms` files, and any file with
  a `VBA` storage containing a `dir` stream, at any depth) is read with
  `read_gms`. The project's root is the storage holding `VBA`. Project
  files saved by drawing suites start with a short header ("GMS", a
  version byte and a few fields, 18 bytes) before the compound file; it
  is skipped (`compound_part`).
- Exported module files (`.bas`, `.cls`, `.frm`) are UTF-8 when they are,
  else in the code page they were exported in, guessed from their bytes
  (Cyrillic, Central European, Greek and the others).
- Checked on four published macro projects (60 modules and their
  forms, Russian comments in code page 1251): the code read from each
  project file is the same, line by line, as the project's exported
  source files, and the form controls land where the form shows them
  (`cargo run -p tracedraw-vba-project --example compare -- file.gms
  folder`; `--example dump -- file.gms` prints the modules and the
  forms' controls).
- The `dir` stream and the modules' code are stored compressed (MS-OVBA
  2.4.1). The crate expands them and also compresses (`ovba::compress`),
  so it can write projects; both follow the chunk and copy token rules of
  the specification, including chunks stored as is.
- From the `dir` stream: the project name, its code page, its
  description and help file, the references (their names and library
  ids) and, for each module, its name (the Unicode name when given), its
  stream, the offset of its source in that stream, whether it is
  procedural, read only or private.
- Each module's code is the compressed text from that offset to the end
  of its stream, decoded in the project's code page (Western, Central
  European, Cyrillic, Greek, Turkish, Hebrew, Arabic, Baltic,
  Vietnamese, Thai, Japanese, Simplified and Traditional Chinese,
  Korean, UTF-8 and others; Western when the page is unknown). A module
  whose source offset is at the end of its stream was saved as compiled
  code only: it comes back with `has_source` false, empty code and a
  warning ("saved as compiled code only, without its source"), and
  `Project::modules_without_source` lists it.
- The PROJECT stream gives each module's kind: `Module=` standard,
  `Class=` class, `BaseClass=` form, `Document=` the host document's
  module. Without it, procedural modules are standard and the others
  classes.
- Protection: the `CMG`, `DPB` and `GC` values are unscrambled (MS-OVBA
  2.4.3). A password (DPB holding more than its single zero byte), a
  project locked for viewing (GC zero) or the protection state's user
  bit set `Project::protected`, with the details in `protection`. The
  code of a protected project is stored in the clear and is read like
  any other.
- `Module::macros` lists a standard module's subs without arguments
  (public or without a scope): what runs as a macro.
  `Project::macros` lists them for the whole project as (module, sub).

### Exported module files

- `read_files` takes files and folders, `read_named` (file name, bytes)
  pairs (for the browser, which has no folders). A folder stands for the
  `.bas`, `.cls`, `.frm` and `.frx` files in it; a compound file among
  the files adds its modules.
- `.bas`: a standard module. `.cls`: a class, its `VERSION 1.0 CLASS`
  header and `BEGIN`/`END` block skipped. `.frm`: a form; its header of
  `Begin`/`End` blocks gives the form's caption and size (`ClientWidth`,
  `ClientHeight` in twips), and for classic forms its controls with
  their position, size, caption, text, colours (`&H...&`), fonts
  (`BeginProperty Font`), enabled, visible, tab order and the rest. A
  forms library form names its controls' file in `OleObjectBlob =
  "name.frx":offset`; the `.frx` file is a compound file holding the
  designer streams, read like a project's form. Without it the form
  keeps its header and the project warns.
- The module name is its `Attribute VB_Name`, else the form block's
  name, else the file name. Text is UTF-8 when it is, else the project's
  code page.

### Forms

- A form's designer storage sits next to `VBA` under the form module's
  stream name. Its `\x03VBFrame` text gives the caption and size; its
  `f` stream the form's own properties (caption, displayed size, back
  and fore colours, font, picture, enabled) and the list of controls,
  with each control's name, id, position (HIMETRIC, read as points),
  tab index, tag, tip, control and row sources, visible, tab stop,
  default and cancel; its `o` stream each control's properties one
  after the other, as many bytes as the control's site gives.
- Control kinds come from the class cache index (MS-OFORMS 2.4.1) or the
  class table's class id: label, text box, command button, check box,
  option button, toggle button, combo box, list box (these six share a
  structure whose display style picks the kind), frame, multi-page,
  page, image, spin button, scroll bar and tab strip. Others are kept as
  `Other(index)`.
- Read per kind: captions, values, group names, sizes, colours, fonts
  (TextProps and standard fonts), enabled, locked, multi-line, word
  wrap, password character, accelerator, maximum length, list style,
  column count, the minimum, maximum, position, steps and orientation
  of spin buttons and scroll bars, the tab captions of tab strips, and
  pictures as their stored bytes.
- A frame or a multi-page keeps its controls (a multi-page its pages,
  each with its own controls) in a storage `i<ID>` (two or more digits)
  under its container; they become the control's `children`.
- Every read is bounds checked: a damaged form keeps what could be read
  and lists the rest in `warnings`, never failing the project.
- `Form::raw` keeps the designer streams as stored, so a form is
  written back unchanged.

### Writing

- `export_module` and `export_project` give `.bas` files, `.cls` files
  (with the class header) and `.frm` files with their `.frx` (the
  designer streams as stored, or written from the form's model), in the
  project's code page with CRLF line ends; an `Attribute VB_Name` line is
  added when the code has none.
- `write_gms` writes a compound file with the VBA project storage:
  every module's code compressed at offset 0 with no compiled code (the
  `_VBA_PROJECT` header's version 0xFFFF, so a host compiles the code
  when it loads it), the `dir`, PROJECT (with the protection values) and
  `PROJECTwm` streams, and the forms' designer storages.

### Limits

- Compiled code (p-code) is not read: a module saved without its source
  has no code to run.
- Forms: the binary layouts were built from the specification and
  checked against forms this crate writes; unusual properties (mouse
  icons, list column widths, multi-page transition effects, tab strip
  tooltips and tags) are skipped, and pictures are kept as bytes in the
  format they were stored in.
- References are names and library ids only; the libraries themselves
  are not loaded.

### Checks (tests in `crates/vba-project`)

- The specification's example without copy tokens expands and compresses
  to the same bytes; a run of 73 bytes is one literal and one copy
  token; text and incompressible noise over several chunks go round
  trip; damaged containers are errors (`ovba::tests`).
- `dir` records with a Cyrillic code page give the names, offsets and
  kinds; a record running past the end is an error (`dir::tests`).
- Scrambled protection values come back for any seed; the module kinds
  and the password, locked and user protection states go round trip
  through a PROJECT stream (`project_stream::tests`).
- A form written with a label, text boxes (one multi-line with
  non-Latin text), a check box, an option button in a group and a
  button reads back with captions, values, positions, sizes, fonts,
  enabled states and group names; a frame's controls in its own storage
  become its children; cut or garbled streams leave warnings, not
  panics (`forms::tests`).
- Classic and forms library `.frm` headers, `.cls` and `.bas` files are
  read (`frm::tests`).
- A project laid out as a host saves it (compiled code before the
  source, references, a document module, a module saved without its
  source, a form) gives its modules, kinds, macros, references and form;
  protection is read while the code stays readable; a Japanese project
  in Shift-JIS keeps its names and code; files that are not projects or
  are damaged give errors, never panics; projects go round trip through
  `write_gms`, through exported module files and through a folder
  (`tests`).

## VBA macros in the app

Macro projects written for other drawing suites load and run in the
desktop app (`apps/tracedraw/src/macros.rs`, `scripting/vba_run.rs`,
`ui/macros_ui.rs`). The browser build has no threads to run them on:
there every macro command (Macro Manager, Run Macro, Macro Editor, Load
Macro Project, a macro in the menu) opens a message that macros run in
the desktop version, with OK and a button to the desktop downloads.

### Running macros

- A macro runs on a worker thread of its own, on a copy of the drawing
  taken when it starts; the app keeps drawing and stays responsive.
- What it changes reaches the drawing at every pause (a message, an
  input box, a form waiting for the user) and when it ends, so a
  macro's changes show while its form is up. All of one run is one
  undo step named "Macro"; if the user edits the drawing during a pause,
  later changes start a new step. Each form event starts from the
  drawing as it is then, so the macro sees the user's edits.
- MsgBox is a modal message box with the style's buttons (OK, OK and
  Cancel, Abort Retry Ignore, Yes No Cancel, Yes No, Retry Cancel), its
  icon (critical, question, exclamation, information) and its default
  button (vbDefaultButton2 to 4), and returns vbOK (1) to vbNo (7).
  Enter picks the default button; Esc and the close box answer Cancel
  when there is one and OK for a lone OK; Yes No and Abort Retry Ignore
  must be answered with a button. The title is "TraceDraw" when the
  macro gives none.
- InputBox shows the prompt, a text field with the default, OK and
  Cancel; Cancel, Esc and the close box return "".
- Debug.Print lines go to the Scripts panel's output.
- While a macro runs, after a moment a notice at the bottom says so with
  a Stop button. Stop, Esc (with no message or modal form open) and
  Tools > Macros > Stop Macro end it at its next statement (a loop
  checks every 256 statements), also while it waits for a message or a
  form. What it changed until then stays, as one undo step, and the
  status line says it was stopped.
- A run-time error stops the run and opens "Run-time error 'n':" with
  the description and "Module m, procedure p, line l" (the VBA line).
  Debug opens the Macro Editor on that line; End closes the box. A
  compile error opens the same box with the module and line.
- Only one macro runs at a time; Run is greyed while one does.
- Settings a macro saves with SaveSetting are kept in
  `settings.json` in the macros folder and read back with GetSetting in
  later runs. `GMSManager.UserGMSPath` is the macros folder.

### Forms on screen

- A form shown with `Show` (modal by default, or `vbModeless`) opens as
  a window over the app with the form's caption and inside size (points
  at 96 dots per inch), back colour, picture and font. A modal form
  blocks the rest of the app until it is hidden or unloaded; a modeless
  one leaves the app usable and its events run while the macro's
  procedure has returned.
- Controls are drawn at their designed places, inside frames and
  multi-page pages, with caption, text, colours and font: labels, text
  boxes (one line or several, password character, maximum length,
  locked), command buttons (the Default button has a blue frame),
  check boxes, option buttons (one per group: same group name, or same
  container), toggle buttons, combo boxes (a drop-down list, or a text
  with a list), list boxes (one choice, several, columns), frames,
  multi-pages and tab strips (the tabs switch pages), images, spin
  buttons and scroll bars. Unknown controls show as an empty box.
- Every property the macro reads or sets goes to the window at once:
  Caption, Value, Text, Enabled, Visible, Locked, Left, Top, Width,
  Height, colours, Font, ListIndex, List, Column, Selected,
  ListCount, Tag, ControlTipText and the rest (`macro_forms.rs`), and
  AddItem, RemoveItem, Clear and SetFocus.
- Events: Click, DblClick, Change, AfterUpdate (a text box left after
  typing), Enter and Exit (focus), KeyDown, KeyUp and KeyPress (a text
  box), SpinUp and SpinDown, Scroll; on the form Click, DblClick,
  Initialize (created), Activate (shown), QueryClose and Terminate.
  Setting a value from the macro fires Change (and Click for check and
  option boxes and lists) as VBA does.
- Enter clicks the form's Default button and Esc its Cancel button
  (outside a text box of several lines).
- `Me.Hide` hides the form and returns from a modal `Show`; `Unload Me`
  removes it. The close box asks the macro through
  `UserForm_QueryClose` (CloseMode vbFormControlMenu); setting Cancel
  keeps the form, otherwise it unloads.
- Forms still open when a run ends or is stopped are closed.

### Macro Manager

- Tools > Macros > Macro Manager opens a docker (also in Window >
  Panels) with the loaded projects as a tree: each project, its modules
  with their kind (module, class, form, document), and each module's
  macros (public Subs without arguments).
- A padlock marks a password-protected project (its code is still
  read) and modules saved without their code, which cannot run.
- Load... (Tools > Macros > Load Macro Project...) takes a `.gms`
  project, or `.bas`, `.cls` and `.frm` modules (the `.frx` beside a
  `.frm` comes along) that become one project. A copy is kept in the
  `macros` folder of the settings folder, and every copy there loads
  again at start-up. A project of the same name replaces the earlier
  copy.
- Run (or a double click on a macro) runs it; Edit opens the editor at
  the module or the macro; Rename renames the project and its copy;
  Unload removes it and deletes its copy; Export writes the chosen
  module as `.bas`, `.cls` or `.frm` with its `.frx`, or the chosen
  project as a `.gms` file.
- Before a project's first run in a session, its warnings are shown
  with Run Anyway and Cancel; they never block. They come from reading
  the code: Windows functions (`Declare`), file statements (Open, Kill,
  FileCopy, Dir...), Shell, objects other than `Scripting.Dictionary`
  given to CreateObject, modules without code, and protection. A "!"
  beside the project shows them too.
- Tools > Macros > Run Macro... (Alt+Shift+F8; Alt+F8 stays the Transformations panel's Rotate) lists every macro of every
  project as "Project > Module.Macro" with a search field; Run, Edit,
  Cancel; Enter runs the chosen one.
- Tools > Macros also lists each project as a submenu of its macros.
  These are commands: Options > Customization > Commands has them under
  the Macros category (and under Tools), so each macro can get a
  shortcut key. The app has no toolbar editing for any command, so
  macros cannot be added to toolbars.

### Macro Editor

- Tools > Macros > Macro Editor... (Alt+F11) opens a window on the
  chosen project (the first one, or a new "Macros" project with a sample
  module when none is loaded): the modules on the left, the code on
  the right with line numbers and VBA colouring (keywords, comments,
  strings).
- Save (Ctrl+S) puts the code back into the module, writes the project
  copy and compiles the project: "Saved. No compile errors." or the
  compile error with its module and line, where the caret goes.
  Switching modules or closing the window saves first.
- Run Sub (F5) runs the Sub the caret is in, after saving.
- Find with Find Next searches from the caret's line on, then from the
  top, case-insensitively, and selects the line found.
- The Scripts panel has a JavaScript / VBA switch: in VBA, Run runs the
  first Sub without arguments of the snippet (or the lines as the body of
  one) as a macro, with its messages, forms and output; Load and Save use
  `.bas` files. The debugger works on JavaScript only.

### Checks (tests in `macros.rs` and `ui/macros_ui.rs`)

- Given a macro that asks with MsgBox (vbYesNo + vbQuestion, title
  "Ask") and InputBox, when the test answers Yes and "Frame", then the
  box shows style 36 and the default text, the rectangle it draws is
  named "Frame", Debug.Print reaches the output, and one undo removes
  the whole run.
- Given a form with a text box and an OK button whose Click draws an
  ellipse named after the text box and unloads the form, when the user
  types "Round" and clicks OK, then UserForm_Initialize has set the
  caption, the ellipse is named "Round" and the macro goes on after
  `Show`.
- A division by zero on line 3 of module Main gives error 11 at Main,
  line 3; an endless loop ends with Stop and the status names it.
- A project written with `write_gms`, loaded from that file, runs and
  counts the page's objects.
- Loading keeps a copy that a new session loads again; Rename renames
  it; Unload deletes it.
- Warnings name the Windows function and the object of another program,
  and leave `Scripting.Dictionary` out.
- MsgBox styles give the right buttons, answers, default button and
  Esc answer.

