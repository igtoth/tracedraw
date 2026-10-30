# Smoothing, smearing and roughening outlines

The shaping brushes of the Shape flyout reshape the outline of an object
where you drag over it, like a sculptor's tool: [[tool.smooth]],
[[tool.smear]], [[tool.twirl]], [[tool.attract_repel]], [[tool.smudge]] and
[[tool.roughen]].

## Using a shaping brush {#using}

1. Select the curves to change. Other objects are converted to curves first.
   The brushes change only the selected objects: with nothing selected a drag
   does nothing, and the status bar says to select an object first.
2. Choose the tool and set its **Nib size** (the radius of the brush, 10 mm
   at first) and its other settings on the property bar.
3. Drag over the outline.

The effect is strongest at the center of the nib and fades to nothing at its
edge. Only the parts of the outline within reach change; open curves stay
open and closed ones stay closed. Each drag is one undo step. The settings
are kept for the next time.

## Smooth {#smooth}

The [[tool.smooth]] tool removes wobbles and extra nodes where you drag.
**Rate** (1 to 100%, 20% at first) sets how strongly.

## Smear {#smear}

The [[tool.smear]] tool (<kbd>W</kbd>) pulls the outline along with the
pointer, as if you smeared wet paint. **Pressure** (1 to 100%, 100% at first)
sets how far it moves.

## Twirl {#twirl}

The [[tool.twirl]] tool turns the outline around the pointer into a swirl.
**Speed** (1 to 100%, 50% at first) sets how fast it turns, and the direction
buttons choose counterclockwise or clockwise; hold <kbd>Alt</kbd> or
<kbd>Shift</kbd> while dragging to turn the other way. Holding the pointer
still keeps turning.

## Attract and Repel {#attract-repel}

The [[tool.attract_repel]] tool pulls the nodes inside the nib towards the
pointer, or pushes them away. **Speed** (1 to 100%, 50% at first) sets how
fast; choose Attract or Repel on the property bar, or hold <kbd>Alt</kbd> or
<kbd>Shift</kbd> while dragging to swap them. Holding the pointer still keeps
going.

## Smudge {#smudge}

The [[tool.smudge]] tool (<kbd>V</kbd>) smooths the outline slightly, then
smears it, so it is pulled without kinks. **Pressure** (1 to 100%, 100% at
first) sets how far.

## Roughen {#roughen}

The [[tool.roughen]] tool (<kbd>E</kbd>) adds jagged spikes along the outline.
**Frequency** (1 to 10 spikes per 10 mm, 5 at first) sets how many, and
**Spike size** (2 mm at first) how tall. The spikes are straight, so they stay
sharp.
