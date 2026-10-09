# Crop, Knife, Segment delete, Eraser

## Crop

- Drag a rectangle. Every object on unlocked layers (or the selection,
  when there is one) is intersected with it: curves become their clipped
  outline (shaping intersect on flattened paths, 0.02 mm), bitmaps keep
  their pixels inside a ClipFrame frame of the crop rectangle, groups
  are cropped member by member. Objects entirely outside are deleted.

## Knife

- Drag a straight cut. Objects crossed by the line (selection, or every
  unlocked object) are split into the part on each side of the line;
  each part keeps the fill and outline and becomes its own object; the
  original is removed. Bitmaps and groups are not cut.

## Segment delete

- Click a segment between two nodes (hit distance 5 screen pixels): the
  segment is removed, splitting the subpath in two open subpaths; a
  closed path becomes open. The object is deleted when nothing is left.

## Eraser (X)

- Thickness: default 6.35 mm (property bar, in the document units).
  Nib: round (default) or square.
- Drag: the band swept by the nib (`shaping::stroke_band`, round caps
  and joins for the round nib, square caps and mitre joins for the
  square one) is subtracted from the selected objects, or from the
  object under the start point when nothing is selected. Objects become
  curves; an object that vanishes completely is deleted. Objects whose
  bounds the band does not touch are left alone.
- Click: erases a dot of the nib's size.
- Double-click an object: deletes it whole.
- Bitmaps, groups and ClipFrames are not erased (the target design
  erases bitmaps by masking; not implemented here).

## Checks

- Given a 40 x 20 mm rectangle and a 4 mm vertical band through its
  middle, when erased, then the object is a path with two subpaths and
  the same bounds.
- Given a dot 100 mm away, then the object is unchanged.
- Given a 100 mm band across the whole rectangle, then the object is
  deleted.
