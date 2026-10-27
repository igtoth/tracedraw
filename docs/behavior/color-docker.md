# Color docker

Window > Panels > Color, Shift+F11, and Outline Colour (Shift+F12).

## Layout

- The top row: three picture buttons that show or hide the views (Color
  Viewers and Color Sliders shown, Color Palettes hidden at first; any
  mix can show), then the model of the sliders: RGB, CMYK (the default),
  HSB, HSL, Lab, Grayscale or YIQ.
- Under it: the selected object's fill colour (a red diagonal when it
  has no solid fill) beside the new colour, the new colour's `#rrggbb`
  (typed values apply when the field is left), the gamut warning when
  the new colour falls outside the CMYK gamut (more than 6 delta E on
  the way there and back), and the eyedropper button, which picks the
  Color Eyedropper tool; after a sample, "Use sampled #rrggbb" takes it.
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
- Color sliders: one per component of the model with its value: RGB and
  YIQ 0 to 255, CMYK, saturation, brightness, lightness and grey 0 to
  100, hues 0 to 360 degrees, Lab L 0 to 100 and a, b -128 to 127.
- Color palettes: a list of the palettes and the chosen one's swatches
  (16 px, at most 512); a click takes the swatch's colour, the tooltip
  names it.
- At the bottom: Fill and Outline apply the new colour to the selection
  (pale with nothing selected); Add to Palette puts it in the document
  palette.

## Following the selection

When the selection changes, the new colour becomes the first selected
object's solid fill, in the model of the sliders. Changes by hand stay
until the selection changes again.

## Checks

- `ui::color_docker::tests::the_new_color_follows_the_selection`
- `ui::color_docker::tests::hue_sliders_count_degrees`
- `ui::color_docker::tests::the_docker_draws_every_view_and_model`

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
