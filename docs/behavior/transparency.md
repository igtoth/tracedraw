# Transparency

| Control | Default | Range |
|---|---|---|
| Type | Uniform | None, Uniform, Gradient (linear, radial, conical, square, shaped), Pattern (two-colour, vector, bitmap), Texture |
| Transparency | 50% | 0..100 |
| Merge mode | Normal | Normal, Add, Subtract, Difference, Multiply, Divide, If lighter, If darker, Texturize, Hue, Saturation, Colour, Invert, And, Or, Xor, Red, Green, Blue |
| Apply to | Fill and outline | fill, outline, both |

Gradient transparency uses the same geometry as gradient fills with
grayscale stops where white = opaque and black = transparent. The
object is rendered to a scratch layer, multiplied by the transparency
mask, then composited with the merge mode. A shaped gradient follows
the object's outline (`tests::shaped_transparency_follows_the_object`).
A gradient mask turns, mirrors and skews with its object as a gradient
fill does (see `gradient-fill.md`, #775).

The Transparency tool shows a gradient mask's nodes as Interactive Fill
shows a fill's (`fill_tool.rs`, #448): start, end and skew handle, the
nodes between and the midpoint slider of each blend; a drag across an
object sets the line (a gradient mask there already keeps its nodes), a
double click on the line adds a node and on a node removes it. Edits are
one undo step "Transparency" per drag; the fill is left alone
(`fill_tool::tests::the_transparency_mask_has_nodes_and_a_midpoint`).

Clear Transparency, a button at the end of the Transparency tool's bar
and of the Properties docker's transparency part (greyed while no
selected object has transparency), and None in the type list take the
opacity back to 100 % and remove the mask, one "Clear Transparency" undo
step (`App::clear_transparency`, #1338,
`eyedropper::tests::clear_transparency_removes_masks_and_opacity`).

Uniform transparency on a group goes on each object it holds, as if set
on each one: members that overlap show through each other, and a
member's own transparency and that of every group around it multiply
(`tests::a_group_with_uniform_transparency_is_drawn_see_through`,
`tests::nested_group_transparencies_multiply`). The canvas, PNG and the
other pictures, SVG (an `opacity` on each member, none on the `<g>`), PDF
and saving `.cdr` all draw it so, and Ungroup multiplies it into each
object, so the objects look as the group did (#1386). The `.cdr` files
seen keep no transparency on a group's own record, only on its objects
(1587 groups in 487 files of versions 7 to 27; a version 11 file holds
groups whose objects all share one transparency, see `cdr-format.md`), so a saved
group comes back as an opaque group of see-through objects that looks
the same.

Uniform transparency with Normal is the object's opacity. With any
other merge mode it is kept as a mask of one grey (its lightness the
opacity, `mixed::uniform_mask`), drawn the same way, so Multiply and the
rest work on uniform transparency too, as in the reference program; the
Transparency part reads it back as Uniform with its amount and merge
mode (`mockup_tests::mockup_steps`).

Freeze is not offered: an earlier version of this page listed it, but it
never existed (#1203).

## Properties docker (#1439, #1203)

The Transparency section of the Properties docker
(`ui/transparency_docker.rs`) lays the settings out top to bottom; the
Transparency tool's bar keeps its one-row form.

1. **Merge mode**, a full-width list at the top.
2. **Type buttons**: No transparency, Uniform, Gradient, Vector pattern,
   Bitmap pattern, Two-colour pattern, Texture, each a picture with a
   tip. The chosen type's title follows ("Uniform Transparency",
   "Gradient Transparency"...). A type button gives the selection that
   type's first preset (a vector pattern of dots, a soft bitmap dot,
   the two-colour checker, clouds); No transparency is Clear
   Transparency.
3. **Transparency picker**: a swatch of the current transparency with a
   drop-down grid of ready-made ones. Uniform offers 0 to 85 % in steps
   of 5 (black at that opacity over a checker), Gradient twelve masks
   (linear both ways, vertical, diagonal, clear-opaque-clear, radial
   both ways, conical, square both ways, shaped, half), the patterns and
   the texture their own tiles (every two-colour tile, five vector and
   four bitmap tiles, every texture at two scales). A click applies the
   whole mask (`TransparencyField::Mask`). The swatches are rendered once
   and kept. **Edit Transparency** beside it switches to the Transparency
   tool to edit the mask on the object.
4. **Uniform**: the amount, a full-width slider (#1438).
5. **Gradient**: buttons for linear, radial, conical, square and shaped;
   the gradient bar with a marker under each stop (a click picks the
   nearest stop, a drag moves it between its neighbours); the picked
   stop's **Opacity** (100 % where the mask is white) and **Position**;
   **+** adds a stop half way to the next one with the gradient's grey
   there, **x** removes the picked stop (two stay); the angle slider for
   linear, conical and square. Stop changes patch only the stops
   (`TransparencyField::MaskStops`), so each object keeps its angle and
   type; a run of changes (a drag) is one undo step.
6. **Two-colour pattern and Texture**: **Foreground transparency** and
   **Background transparency** (0 % opaque, 100 % invisible; the mask's
   two greys, white opaque) and **Swap**
   (`TransparencyField::MaskColors`).
7. **Transparency target**: three buttons, Fill, Outline, All.
8. **Clear Transparency**, as before.

Settings that differ across the selection show as mixed; a change sends
only that setting to each object (`mixed::patch_transparency`).

Not yet: the ten missing merge modes, Free scale and skew, the mirror
tiles button, foreground and background for vector and bitmap patterns,
the texture library list, saving a new preset from the picker, and the
docker header's Scroll/Tab mode and style indicators.
