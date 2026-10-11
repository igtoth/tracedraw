# Rulers, grids and guidelines

## Rulers

- 17 px rulers along the top and the left of the drawing window, `#F4F4F4`
  with a 1 px `#D8D8D8` border on the canvas side. The horizontal ruler runs
  over the vertical scrollbar, the vertical one down beside the bottom row.
- Numbered marks are 1, 2 or 5 times a power of ten ruler units, the
  smallest step that keeps them at least 44 px apart. Each number is
  centred over a 5 px tick; the half-way tick is 2 px; the others are 1 px.
  Ticks end one pixel above the border.
- Ticks per numbered step: the tick divisions setting (10 by default),
  reduced to its half, then 5, 4, 2 or 1 when ticks would be closer than
  3 px.
- Numbers are distances from the ruler origin without sign (200 100 0 100
  200), 10 px grey, with as many decimals as the step needs.
- The unit name ("millimeters", "pixels") sits at the right end of the
  horizontal ruler; numbers that would run into it are left out.
- The pointer position shows on both rulers as a dark dashed line (3 px
  on, 3 off).
- The corner where the rulers meet holds the origin button: a dotted
  cross with a handle. Dragging out of it moves the ruler origin to where
  it is released inside the drawing window (snapped like objects),
  showing dashed lines across the window while dragging.
- Dragging out of a ruler adds a guideline (horizontal from the top ruler,
  vertical from the left one) where it is released inside the window.
- Double-clicking a ruler or the corner opens Document Options at Rulers.

### Origin

The origin is saved with the drawing (mm from the page's bottom-left
corner, 0, 0 by default). Rulers, the status bar's cursor coordinates,
the property bar's X and Y, the Transformations summary, the Guidelines
panel and the document grid all count from it.

### Document Options > Rulers

Units (inches, millimeters, picas,
points, pixels, ciceros, didots, feet, yards, miles, centimeters, meters,
kilometers), Origin (horizontal, vertical), Tick divisions, Nudge (nudge
2.54 mm, super nudge 5.08 mm with Shift, micro nudge 0.254 mm with Ctrl),
Show rulers, and a button that puts the origin back at the page corner.

## Document grid

- View > Grid > Document Grid shows it over the page and the desktop,
  behind objects, as 1 px `#DCDCDC` lines or `#8C8C8C` dots at the
  intersections (Document Options > Grid > Show grid as).
- Spacing: horizontal and vertical, 10 mm by default, shown either as a
  distance ("millimeters apart") or as lines per unit. Lines pass through
  the ruler origin.
- When lines would be closer than 6 px (8 px for dots), every 2nd, 5th,
  10th, 20th... line is drawn instead.
- Snap to grid uses the same spacing and origin.

## Baseline grid

- Lines across the page like a ruled notebook: the first one "start from
  top" below the page top (12.7 mm), then every 14 pt down to the page
  bottom, in the line colour (light blue by default).
- View > Snap To > Baseline Grid snaps to them; Text > Align to Baseline
  Grid uses the spacing.

## Pixel grid

- One cell per document pixel (the document resolution), aligned with
  the page's bottom-left corner, drawn over objects in the Pixels view (or
  with pixel rulers) from 800% zoom when "Show grid at 800% or higher
  zoom" (View > Grid > Pixel Grid) is on, which it is by default.
- Colour (light grey) and opacity (100%) are document settings.
- In the Pixels view, 100% zoom is one screen pixel per document pixel,
  as the zoom box shows it; otherwise 100% is real size at 96 dpi.

## Guidelines

- Guidelines run across the window, dashed by default, in their own
  colour or the document's default guideline colour (blue). Selected
  guidelines are red.
- Each guideline has a colour, a style (solid, dashed, dotted, dash dot)
  and a lock, saved with the drawing.
- Pick tool: a click selects a guideline (Shift adds or removes); a press
  and drag moves it, snapping to the grid, the page and objects; dropping
  it outside the drawing window deletes it. The move is one undo step.
- A click on the only selected guideline shows its rotation handles: a
  pivot where it was clicked and two curved arrows 70 px along it.
  Dragging an arrow turns the guideline about the pivot in whole degrees;
  0 and 180 make it horizontal, 90 and 270 vertical.
- Locked guidelines can be selected but not moved or deleted.
- Delete removes the selected guidelines that are not locked.
- Right-click on a guideline: Undo, Delete, Lock or Unlock, Properties
  (opens the Guidelines panel with the guideline's values).
- Edit > Select All > Guidelines selects every guideline of the page.

### Guidelines panel

Show and Snap toggles, Guideline type (horizontal, vertical, angled), the
x, y and angle fields (counted from the ruler origin), Add and Modify, the
list of the page's guidelines of that type (Ctrl+click selects several),
Delete, Lock or Unlock, and the colour and style of the selected
guidelines (the default colour when none is selected).

### Document Options > Guidelines

Show guidelines, Snap to guidelines, the default guideline colour, the
default preset guideline colour, and presets added in the preset colour:

- Presets: one inch margins, bleed area, page borders, printable area,
  three column newsletter (one inch margins and three columns 1/4 inch
  apart), basic grid (one inch squares), upper left grid (4 x 4 over the
  upper left quarter).
- User defined: margins (top, bottom, left, right), columns (number,
  distance between, set between the margins when those are on), grid
  (rows, columns).

A preset line already present is not added twice.

## Save as Default

Tools > Save Settings as Default (and the button in Options) keeps the
active drawing's grid, ruler and guideline settings for new drawings.

## Checks

- `ui::rulers::tests::numbered_marks_keep_their_distance`
- `ui::rulers::tests::ticks_follow_the_tick_divisions_and_thin_out`
- `ui::rulers::tests::labels_have_no_sign_and_just_enough_decimals`
- `guides::tests::dragging_a_guide_is_one_undo_step_and_leaves_other_guides_alone`
- `guides::tests::dropping_a_guide_outside_the_window_deletes_it`
- `guides::tests::locked_guides_select_but_do_not_move_or_delete`
- `guides::tests::second_click_rotates_a_guide_about_the_clicked_point`
- `ui::layout_options::tests::*` (form origin, presets)
- `canvas::tests::grid_lines_thin_out_and_follow_the_origin`
- `tracedraw_core` document tests: old guidelines load, appearance round
  trips, defaults for documents without grid settings.
