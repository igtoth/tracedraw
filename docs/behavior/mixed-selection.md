# Editing several objects with different values

With several objects selected, the Properties panel (Outline, Fill,
Transparency and Rectangle sections, so also the Outline Pen tool) and
the property bar (outline part, Interactive Fill bar, rectangle part,
Transparency part) show the first object's values. A value that is not
the same on every selected object shows as mixed, and a change sends
only the attribute that was changed: each object keeps its own values
for everything else. A group stands for the objects inside it.

## What a mixed value looks like

| Control | Mixed |
| --- | --- |
| Number field (outline width, corner size, gradient angle, edge pad, transparency angle) | empty; the hover text says the objects differ |
| Colour button (fill colour, outline colour, gradient start and end) | an empty swatch with a slash and the word "Mixed" in the panel |
| Button rows and lists (fill type, caps, corners, dash, arrowheads, corner style, gradient type, transparency type, merge mode, what it applies to) | nothing highlighted, the list shows no value |
| Check box (outline on, hairline, behind fill, scale with object, relative corner scaling) | neither on nor off |
| Slider (nib stretch and angle, uniform transparency) | the first object's value, with "(Mixed)" in its text |
| Fill types that differ | no type highlighted and no settings until a type is picked |
| A gradient, pattern, texture or mesh whose settings differ | the first object's editor under the word "Mixed" |

A value typed or picked in a mixed control goes to every selected
object, also when it equals the value the first object has.

The status bar's fill and outline read "Mixed" (no swatch) the same way.

## What a change sends

| Edited | Each object gets | Keeps |
| --- | --- | --- |
| Outline width, colour (or the outline's own fill), caps, corners, dash, arrowheads, nib stretch, nib angle, behind fill, scale with object | that one value | its other outline values |
| Outline turned on | the default outline, when it has none | its outline, when it has one |
| Outline turned off | no outline | |
| Fill type, uniform colour, pattern, texture or mesh | the whole fill | its outline and transparency |
| A gradient's type, start or end colour, stops, angle, centre, edge pad, repeat, mirror, reverse | that setting in its own gradient (an object without a gradient gets the whole one) | its other gradient settings |
| Transparency type | that type with the default mask, unless it has that type already | its merge mode and target |
| Uniform amount | uniform transparency at that amount | |
| Transparency angle or gradient type | that setting in its own gradient mask | its other mask settings |
| Transparency stops (the docker's list) | the whole run of stops in its own gradient mask (an object without one gets the whole mask) | its angle, type and other mask settings |
| Transparency foreground or background | the two greys in its own pattern or texture mask (an object without one gets the whole mask) | its tile or texture |
| Merge mode, applies to | that setting | its mask and amount |
| Corner style, a corner size, relative scaling | that setting | its other corner values |

An object without an outline that takes an outline value gets the
default outline with that value. Every change is one undo step.

The property bar's outline width box, line style and arrowhead lists,
and an outline colour picked from a palette, work the same way: each
object, and each object inside a selected group, keeps its own values
for the rest of its outline. The line style and arrowheads leave objects
without an outline as they are.

Commands: `Command::PatchStroke` and `Command::PatchFill` carry the
fields to copy (`tracedraw_core::patch::StrokeField`, `FillField`);
`patch::stroke_mixed` and `patch::fill_mixed` say which values differ.

Checks: `patch::tests::*` (core), `mixed::tests::*` (a width change
keeps both colours, a colour change keeps both widths, each outline
attribute alone, the property bar width and colour (also in a group), a value typed
into a mixed field, gradient and transparency settings one at a time,
mixed corners) and `ui::dockers::tests::mixed_values_show_empty`.
