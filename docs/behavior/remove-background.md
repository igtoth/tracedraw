# Remove Background (on this computer)

Bitmaps > Remove Background... (`menu.bitmaps.remove_background_local`,
also on the bitmap property bar and in a bitmap's context menu) opens a dialog that makes the background of
the selected bitmaps transparent without an AI service
(`bg_remove::remove_background`). The AI command stays as Remove
Background with AI (`menu.bitmaps.remove_background`).

## Dialog

A window the user sizes: the shared Before and After preview at the left
(`before-after-preview.md`: the Preview list, zoom and pan, a
checkerboard where transparent), and at the right:

| Setting | Range | At first | Meaning |
|---|---|---|---|
| Background | colours | learnt from the border | up to four background colours; the eyedropper adds the colour under a click on a preview (from the picture before the change), a swatch click removes one, Automatic forgets them |
| Tolerance | 0 to 100 (Delta E) | 14 | how far a colour may be from a background colour and still be background |
| Edge softness | 0 to 20 px | 1 | feather of the edge, in pixels of the bitmap (previews scale it) |
| Contiguous | on/off | on | only background connected to the border goes (through a backdrop that drifts in colour too, see Algorithm) |
| Remove specks | on/off | on | loose kept spots under 1/2000 of the picture (at least 12 px) go |
| Clean edge colors | on/off | on | half-transparent edge pixels lose the backdrop's colour |

Reset puts every setting back. OK changes every selected bitmap in one undo
step (`apply_to_bitmaps`); the "?" opens `bitmaps/remove-background`.

## Algorithm

1. Background colours: the opaque pixels of a two pixel border, sampled
   to at most 4096, clustered in CIE Lab by k-means (farthest-point seeds,
   a new seed only when at least 10 Delta E from the others, at most six,
   ten rounds); a cluster is kept when it holds at least a fifteenth of
   the border. When fewer than a third of the border pixels are opaque the
   picture already has a transparent background and is left alone.
2. Each pixel's distance is the CIE76 Delta E to its nearest background
   colour (transparent pixels: 0).
3. Opacity: 0 up to the tolerance, a smoothstep ramp over a band of
   max(4, 0.6 x tolerance) past it, 1 beyond.
4. Contiguous: a breadth-first fill from the border pixels reaches pixels
   under tolerance + band; it passes freely through pixels within the
   tolerance and at most 2 + ceil(softness) pixels into the ramp, so it
   takes the anti-aliased rim but not the subject. From a background
   pixel it also walks on to a neighbour whose colour differs from it by
   at most max(3, tolerance / 3) Delta E, as long as that neighbour is
   within 3 x tolerance of a background colour: a backdrop that drifts
   away from what the border showed (a texture, a gradient, a soft
   shadow under the subject) goes whole, pixel by small step, while the
   subject's edge, a sharp step, stops the walk (its anti-aliased rim is
   taken by the ramp as before). Pixels reached by such steps are
   background whatever their distance. Unreached pixels stay opaque.
   Off: every pixel gets the ramp's opacity.
5. Remove specks: 4-connected kept parts (opacity at least 0.5) smaller
   than the limit go.
6. Edge softness: two box blurs of the opacity with the radius in pixels.
7. Clean edge colors: for 0.05 < opacity < 1, the colour is solved from
   C = a F + (1 - a) B with B the nearest background colour, clamped.
8. The new opacity multiplies the pixel's own alpha.

## Checks

- `bg_remove::tests::the_backdrop_goes_and_the_subject_stays`: a gradient
  backdrop goes at both ends, the subject keeps its colour, a
  background-coloured dot inside it stays, a loose speck goes.
- `bg_remove::tests::everywhere_mode_takes_background_coloured_parts_inside_too`
- `bg_remove::tests::edge_pixels_lose_the_backdrops_colour`
- `bg_remove::tests::learnt_colours_cover_a_two_tone_backdrop`
- `bg_remove::tests::picked_colours_and_tolerance`
- `bg_remove::tests::a_transparent_picture_is_left_alone`
- `bg_remove::tests::feathering_softens_only_the_edge`
- `bg_remove::tests::a_drifting_backdrop_goes_and_the_subject_stays`: a
  soft shadow 36 Delta E from the white backdrop, over a faint texture,
  goes with the backdrop; the subject keeps its edge; Everywhere mode
  goes by colour alone.
- `ui::dialogs::tests::every_dialog_draws_and_accepts_enter` draws the
  dialog.
