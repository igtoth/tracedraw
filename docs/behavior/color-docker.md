# Color docker

Window > Panels > Color, Shift+F11, and Outline Colour (Shift+F12).

## Layout

- The top row, one line: three small picture buttons on the left that
  show or hide the views (Color Viewers and Color Sliders shown, Color
  Palettes hidden at first; any mix can show), the model list across the
  rest of the width (RGB, CMYK (the default), HSB, HSL, Lab, Grayscale or
  YIQ) and the gear on the right. The gear's menu ticks the three views
  and the fill-and-outline lock, and Reset panel puts the views, the
  model (CMYK), the tint (100 %), the lock and the viewer's hue back to
  their defaults.
- Color viewer: a square with saturation from left (0) to right (100 %)
  and brightness from bottom (0) to top (100 %) for the current hue, and
  the hue strip beside it (red at the top, then yellow, green, cyan,
  blue, magenta, red). Click or drag in either. A grey keeps the last hue
  shown; picking a hue for a grey takes full saturation and at least
  half brightness so the hue shows. In CMYK (the docker's model, or a
  CMYK colour in the colour picker and the fill editors) the square spans
  the inks instead (#907): left to right the hue's inks go from 0 to
  100 %, top to bottom black ink goes from 0 to 100 %, and the square and
  the hue strip show how those inks print. The hue's inks are the
  complement of its pure screen colour (red is M 100 Y 100, orange
  M 50 Y 100), so the corners are paper, the solid inks, K 100 and the
  inks over K 100 (at most 300 % ink), and the ring sits on any CMYK
  colour's inks (black ink for the height, the strongest of C, M and Y
  for the width) and reaches every edge. In the other models the square
  is the screen's saturation and brightness.
- Color sliders: one row per component of the model, its letter, the
  slider and its value box: RGB and YIQ 0 to 255, CMYK, saturation,
  brightness, lightness and grey 0 to 100, hues 0 to 360 degrees, Lab L
  0 to 100 and a, b -128 to 127. Each slider's rail is the gradient of
  its component with the other components as they are (no cyan to full
  cyan for C, the spectrum for a hue, black to white for grey), the
  colours each point would give, in the model's own look (CMYK through
  the press model). The same sliders serve the colour picker popup and
  the fill editors. The rail's colours are kept between frames while
  the other components stay the same.
- Hex: the new colour's `#rrggbb` in a small labelled box (typed values
  apply when the field is left) and the gamut warning when the new
  colour falls outside the CMYK gamut (more than 6 delta E on the way
  there and back). The row is always shown, under the sliders.
- Color palettes: a list of the palettes and the chosen one's swatches
  (16 px, at most 512), with Tint (0 to 100 %, the tint of the spot
  inks) under them; a click takes the swatch's colour: a spot colour at
  that tint of its ink (100 % is the swatch itself), any other colour as
  it is, so a tint set for an ink never lightens a plain swatch. The
  panel shows one Tint control: under the swatches while the palette
  view shows, else under the spot inks. The tooltip names the swatch.
- The strip at the bottom: the lock, the fill-over-outline swatch (the
  new colour as the fill in front, the first selected object's outline
  colour behind it, a red diagonal for none; with nothing selected the
  outline colour new objects get; the tip gives both codes), the
  eyedropper button, which picks the Color Eyedropper tool, and the
  "..." button that drops the other actions (see below). Under the
  swatch, Fill and Outline (small, pale with nothing selected) apply
  the new colour to the selection. After a sample, "Use sampled
  #rrggbb" under them makes it the new colour.

## The lock and the other actions

- The lock (the padlock at the left of the strip, off at first, kept
  while the application runs) ties fill and outline together: while it
  is on, Fill and Outline each give the selected objects the new colour
  as fill and as outline colour, in one undo step ("Fill and Outline").
  Off, each changes its own side.
- The "..." menu:
  - No Fill, No Outline: take the selection's fill or outline away (the
    same as the property bar's and the palettes' none swatches; with
    nothing selected they ask about the defaults of new objects).
  - Swap Colors: in one undo step, each selected object's fill colour
    becomes its outline colour and its outline colour its solid fill,
    objects inside groups each on their own. An object with no outline
    ends with no fill, one with no fill with no outline; a fill that is
    not one colour (a gradient, a pattern) gives the outline the colour
    it shows in previews. The drawing is walked once, so thousands of
    objects swap without a pause. The current colours the palettes'
    clicks remember are left as they are (Undo could not give them back).
  - Add to Document Palette: puts the new colour in the document palette
    (nothing when it is there already; its swatch gets the frame of the
    last colour used).
  - Add to Custom Spot Colors: adds a spot ink standing for the new
    colour in process inks, named as typed in Ink name, else by the
    colour's `#rrggbb` code; a name the drawing has already gets a
    number (`#0a141e 2`). Ink name is cleared.
  - Add to Color Styles Palette: adds a colour style of the new colour,
    named "Color N" like New color style in the Color Styles panel.

The eyedropper samples objects of the drawing only: there is no
screen-wide sampling, since reading pixels outside the window needs a
system screen capture (a portal permission on Wayland, a screen
recording permission on macOS) and its platform libraries, and the
browser build has no way to it at all.

## Following the selection

When the selection changes, or the first selected object's solid fill
goes to another colour model (Tools > Convert Colors), the new colour
becomes that fill. A CMYK or RGB fill brings the sliders to its own
model, so a fill converted to RGB shows its RGB values (#1254); another
model's fill shows in the model of the sliders. A fill changed within
its model (a colour applied, an undo) leaves the new colour alone.
Changes by hand stay until the selection changes or its fill is
converted.

## Checks

- `ui::color_docker::tests::the_new_color_follows_the_selection`
- `ui::color_docker::tests::the_new_color_follows_a_converted_fill_into_its_model`
- `ui::color_docker::tests::hue_sliders_count_degrees`
- `ui::color_docker::tests::the_docker_draws_every_view_and_model`
- `pick_drag_tests::the_color_panel_menu_swaps_fill_and_outline`
- `pick_drag_tests::the_color_panel_menu_takes_the_fill_and_the_outline_away`
- `pick_drag_tests::the_color_panel_menu_adds_the_new_color_to_the_palette_the_inks_and_the_styles`
- `pick_drag_tests::the_color_panel_lock_applies_the_color_to_fill_and_outline_at_once`
- `ui::color_docker::tests::slider_rails_run_through_their_component`
- `ui::color_docker::tests::a_palette_colour_is_taken_at_the_tint`
- `ui::color_docker::tests::reset_panel_puts_the_defaults_back`

## The colour picker popup

Every colour of the drawing is picked with this one popup (#442): the
swatches of the Properties docker (fill, outline, the extra fills and
outlines, the text's character colours), the Interactive Fill bar
(uniform colour, gradient start, end and picked node, the palette
menu's other colour), the Text docker's character fill, outline and
frame fill, the shadow and block shadow colours, the gradient stops of
the fill editors, the table bar's cell fill and borders, the Border and
Grommet colour, the palette editor and the page background. It opens
beside the swatch (`ui/color_picker.rs`): the viewer (saturation and brightness
square with the hue strip), a **Model** list and the sliders of the
colour's model. The colour is edited in its own model, a CMYK colour in
CMYK and an RGB one in RGB; choosing another model in the list converts
the colour and the object holds a colour of that model from then on,
with the sliders changing to it. The list offers the docker's models
(RGB, CMYK, HSB, HSL, Lab, Grayscale, YIQ) and, after them, the
registration colour (it prints on every separation; the list shows it
as chosen for that colour, #286,
`ui::color_picker::tests::the_model_list_offers_the_registration_colour`);
a spot ink or registration colour is edited in CMYK until another model
is chosen. Edit Fill's uniform page has the same list. Esc or a click
outside closes the popup; a click on the Model list's entries that reach
below the popup (Lab, Grayscale, YIQ) chooses them, it does not count
as outside (#440,
`ui::color_picker::tests::a_choice_in_the_model_list_keeps_the_popup_open`,
`ui::color_picker::tests::the_model_can_be_changed_on_demand`).

A drag in the popup on the Properties docker's or the Interactive Fill
bar's uniform colour is one undo step, and the colour joins the
document palette when the button is let go, not every colour passed
over (#442).
