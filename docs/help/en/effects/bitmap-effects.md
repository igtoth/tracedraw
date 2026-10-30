# Bitmap effects

Bitmap effects change pixels: blur, sharpen, adjust colors, add textures and
artistic strokes. They work on bitmaps and also on vector objects, which are
then drawn as the picture the effects make while keeping their own shape,
fill and outline.

Effects are kept apart from the original pixels: you can edit, hide, reorder
or remove any of them later, and removing the last one gives the original
back.

## To apply a bitmap effect {#apply}

1. Select one or more objects.
2. Choose a group in the [[menu.effects]] menu and an effect.
3. Effects without settings apply at once. The others open a dialog box you
   can size, with the first selected object before and after in the
   [Before and After preview](../bitmaps/before-after-preview.md) and the
   settings on the right. The effect also shows on the drawing and follows
   every value as it changes (untick **Preview** to switch that off); the
   work is done in the background, so the window keeps answering on large
   pictures. Click [[dialog.ok]] to keep what the drawing shows as one undo
   step; **Cancel** or <kbd>Esc</kbd> put the drawing back exactly as it was.
   The settings sit in a panel at the right; under it **Reset** puts every
   setting back to its default, and the two arrows undo and redo the changes
   made in the dialog box.

![A bitmap effect dialog box](../../img/effects-bitmap-effects.png)

The change to the whole selection is one undo step. With a bitmap selected,
<kbd>Ctrl+B</kbd> opens [[fx.brightness_contrast_intensity]], <kbd>Ctrl+Shift+B</kbd>
[[fx.color_balance]] and <kbd>Ctrl+Shift+U</kbd> [[fx.hue_saturation_lightness]].

## The effect groups {#groups}

| Group | Effects |
|---|---|
| [[menu.effects.3d]] | [[fx.cylinder]], [[fx.emboss]], [[fx.page_curl]], [[fx.pinch_punch]], [[fx.sphere]] |
| [[menu.effects.adjust]] | [[fx.auto_adjust]], [[fx.image_adjustments]], [[fx.contrast_enhancement]], [[fx.local_equalization]], [[fx.target_balance]], [[fx.tone_curve]], [[fx.brightness_contrast_intensity]], [[fx.color_balance]], [[fx.gamma]], [[fx.hue_saturation_lightness]], [[fx.selective_color]], [[fx.replace_colors]], [[fx.desaturate]], [[fx.channel_mixer]] |
| [[menu.effects.art_strokes]] | [[fx.charcoal]], [[fx.conte_crayon]], [[fx.crayon]], [[fx.cubist]], [[fx.impressionist]], [[fx.palette_knife]], [[fx.pastels]], [[fx.pen_ink]], [[fx.pointillist]], [[fx.scraperboard]], [[fx.sketch_pad]], [[fx.watercolor]], [[fx.felt_marker]], [[fx.paper_grain]] |
| [[menu.effects.blur]] | [[fx.directional_smooth]], [[fx.gaussian_blur]], [[fx.jagged_edges]], [[fx.low_pass]], [[fx.motion_blur]], [[fx.radial_blur]], [[fx.smart_blur]], [[fx.smooth]], [[fx.soften]], [[fx.zoom]] |
| [[menu.effects.camera]] | [[fx.colorize]], [[fx.diffuse]], [[fx.photo_filter]], [[fx.sepia_toning]], [[fx.vintage_photo]] |
| [[menu.effects.color_transform]] | [[fx.bit_planes]], [[fx.halftone]], [[fx.psychedelic]], [[fx.solarize]] |
| [[menu.effects.contour_group]] | [[fx.edge_detect]], [[fx.find_edges]], [[fx.trace_contour]] |
| [[menu.effects.correction]] | [[fx.dust_and_scratch]] |
| [[menu.effects.creative]] | [[fx.crystallize]], [[fx.fabric]], [[fx.frame]], [[fx.glass_tiles]], [[fx.mosaic]], [[fx.scatter]], [[fx.tinted_glass]], [[fx.stained_glass]], [[fx.vignette]], [[fx.vortex]] |
| [[menu.effects.custom]] | [[fx.bump_map]] |
| [[menu.effects.distort]] | [[fx.blocks]], [[fx.displace]], [[fx.mesh_warp]], [[fx.offset]], [[fx.pixelate]], [[fx.ripple]], [[fx.swirl]], [[fx.tile]], [[fx.wet_paint]], [[fx.whirlpool]], [[fx.wind]] |
| [[menu.effects.noise]] | [[fx.add_noise]], [[fx.maximum]], [[fx.median]], [[fx.minimum]], [[fx.remove_moire]], [[fx.remove_noise]] |
| [[menu.effects.sharpen]] | [[fx.adaptive_unsharp]], [[fx.directional_sharpen]], [[fx.high_pass]], [[fx.sharpen]], [[fx.unsharp_mask]] |
| [[menu.effects.texture]] | [[fx.cobblestone]], [[fx.wrinkles]], [[fx.etching]], [[fx.plastic]], [[fx.relief_sculpture]], [[fx.stone]] |
| [[menu.effects.transform]] | [[fx.deinterlace]], [[fx.invert_colors]], [[fx.posterize]] |

## Special dialogs {#special-dialogs}

- **[[fx.image_adjustments]]** opens a larger dialog box: a large preview you can
  zoom, pan and rotate, shown alone, side by side or split; **Auto adjust**
  and pickers for the white and black points; sliders for temperature, tint,
  saturation, brightness, contrast, highlights, shadows and midtones, with a
  histogram; undo and redo inside the dialog; and snapshots of settings to
  compare.
- **[[fx.contrast_enhancement]]** shows the histogram with markers to drag for the
  input and output levels, eyedroppers for black and white, and **Auto
  adjust**.
- **[[fx.tone_curve]]** edits a curve per channel (master, red, green, blue) in four
  styles (curve, straight, freehand, gamma), over the histogram; curves can
  be saved and loaded.
- Colors of the picture, such as in [[fx.replace_colors]], have an eyedropper: click
  a preview to take the color under the pointer.

## Editing the effects of an object {#fx-section}

The [[docker.fx]] section of the [[docker.properties]] panel lists the
effects of the selected object, first applied at the top. Each row has a
check box to show or hide the effect, **Edit** (opens its dialog box),
**Move up**, **Move down** and **Delete**. **Add effect** opens the effect
groups.

[[menu.effects > menu.effects.flatten]] makes the effects permanent: a bitmap
keeps what it shows, and a vector object becomes a bitmap.

## The Effect Picker {#effect-picker}

[[menu.effects > menu.effects.picker]] opens a window that stays open while
you work, with every effect as a small picture. Type in the search field to
find effects, or choose a group or **Recent** from the list. With something
selected, click a tile to apply the effect. You can also apply an effect as a
[lens](lens.md).

## Effects that spread past the edges {#inflate}

Blurs and similar effects can reach past the edges of a bitmap. With
[[menu.bitmaps > menu.bitmaps.inflate > menu.bitmaps.inflate_auto]] on (as at
first), the bitmap grows to make room for them. See
[Resampling and straightening](../bitmaps/resample-and-straighten.md#inflate).

To save a bitmap with its effects as a PNG file, right-click it and choose
[[menu.bitmaps.save_bitmap_as]].
