# Transparency

| Control | Default | Range |
|---|---|---|
| Type | Uniform | None, Uniform, Gradient (linear, radial, conical, square, shaped), Pattern, Texture |
| Transparency | 50% | 0..100 |
| Merge mode | Normal | Normal, Add, Subtract, Difference, Multiply, Divide, If lighter, If darker, Texturize, Hue, Saturation, Colour, Invert, And, Or, Xor, Red, Green, Blue |
| Apply to | Fill and outline | fill, outline, both |
| Freeze | off | |

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
