//! The object model VBA macros use, driven the way compiled macros call
//! it: exact member names, 1-based items, lengths in the document unit.

use crate::app::App;
use crate::scripting::run_vgcore;
use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::Rect,
    Color, Fill, ShapeId,
};

fn run(app: &mut App, src: &str) -> Vec<String> {
    match run_vgcore(app, src, "Macro") {
        Ok(out) => out,
        Err(e) => panic!("macro failed: {e}"),
    }
}

/// Every object on the active page, groups' insides too.
fn all(app: &App) -> Vec<Shape> {
    fn walk(s: &Shape, out: &mut Vec<Shape>) {
        out.push(s.clone());
        if let ShapeKind::Group { children } = &s.kind {
            children.iter().for_each(|c| walk(c, out));
        }
    }
    let mut out = Vec::new();
    for p in &app.doc().pages {
        for l in &p.layers {
            l.shapes.iter().for_each(|s| walk(s, &mut out));
        }
    }
    out
}

fn page_bounds(app: &App, id: ShapeId) -> Rect {
    app.doc().shape_on_page(id).expect("shape").bounds()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn lengths_follow_the_document_unit() {
    let mut app = App::headless();
    let out = run(
        &mut app,
        "ActiveDocument.Unit = cdrMillimeter;
         const r = ActiveLayer.CreateRectangle(10, 30, 50, 10);
         r.Name = 'mm';
         ActiveDocument.Unit = cdrInch;
         const s = ActiveLayer.CreateRectangle2(1, 1, 2, 1);
         s.Name = 'in';
         print(s.SizeWidth, s.SizeHeight, r.SizeWidth);",
    );
    let mm = all(&app)
        .into_iter()
        .find(|s| s.name.as_deref() == Some("mm"))
        .expect("mm");
    let b = page_bounds(&app, mm.id);
    assert!(
        close(b.x0, 10.0) && close(b.y0, 10.0) && close(b.x1, 50.0) && close(b.y1, 30.0),
        "{b:?}"
    );
    let inch = all(&app)
        .into_iter()
        .find(|s| s.name.as_deref() == Some("in"))
        .expect("in");
    let b = page_bounds(&app, inch.id);
    assert!(
        close(b.x0, 25.4) && close(b.width(), 50.8) && close(b.height(), 25.4),
        "{b:?}"
    );
    let v: Vec<f64> = out[0]
        .split(' ')
        .map(|x| x.parse().expect("number"))
        .collect();
    assert!(
        close(v[0], 2.0) && close(v[1], 1.0) && close(v[2], 40.0 / 25.4),
        "{out:?}"
    );
    // One undo step for the whole run.
    assert_eq!(app.engine.undo_label(), Some("Macro"));
}

#[test]
fn the_reference_point_places_and_scales() {
    let mut app = App::headless();
    run(
        &mut app,
        "ActiveDocument.Unit = cdrMillimeter;
         const r = ActiveLayer.CreateRectangle2(40, 50, 20, 10);
         r.Name = 'r';
         ActiveDocument.ReferencePoint = cdrBottomLeft;
         r.SetPosition(0, 0);
         r.SetSize(30, 0);
         ActiveDocument.ReferencePoint = cdrTopRight;
         print(r.PositionX, r.PositionY);",
    );
    let r = all(&app)
        .into_iter()
        .find(|s| s.name.as_deref() == Some("r"))
        .expect("r");
    let b = page_bounds(&app, r.id);
    // Placed at the origin, then 30 wide keeping the proportions, about
    // the lower left corner.
    assert!(close(b.x0, 0.0) && close(b.y0, 0.0), "{b:?}");
    assert!(close(b.width(), 30.0) && close(b.height(), 15.0), "{b:?}");
}

#[test]
fn collections_are_one_based_named_iterable_and_reach_into_groups() {
    let mut app = App::headless();
    let out = run(
        &mut app,
        "ActiveDocument.Unit = cdrMillimeter;
         const L = ActiveLayer;
         const a = L.CreateRectangle2(0, 0, 10, 10); a.Name = 'A';
         const b = L.CreateEllipse2(30, 30, 5); b.Name = 'B';
         const c = L.CreateRectangle2(50, 0, 10, 10); c.Name = 'C';
         const sr = new ShapeRange(); sr.Add(b); sr.Add(c);
         const g = sr.Group(); g.Name = 'G';
         let names = [];
         for (const s of ActivePage.Shapes) names.push(s.Name);
         print(ActivePage.Shapes.Count, ActivePage.Shapes.Item(1).Name, ActivePage.Shapes.Item('G').Type === cdrGroupShape);
         print(names.join(','));
         print(g.Shapes.Count, g.Shapes.Item(2).Name);
         print(ActivePage.Shapes.FindShapes('', cdrEllipseShape, true).Count, ActivePage.Shapes.FindShapes('', cdrEllipseShape, false).Count);
         print(ActivePage.FindShapes('', 0, true, \"@type = 'rectangle' and @name <> 'A'\").Item(1).Name);",
    );
    assert_eq!(out[0], "2 A true");
    assert_eq!(out[1], "A,G");
    assert_eq!(out[2], "2 C");
    assert_eq!(out[3], "1 0");
    assert_eq!(out[4], "C");
}

#[test]
fn colours_and_outlines_change_the_object() {
    let mut app = App::headless();
    let out = run(
        &mut app,
        "ActiveDocument.Unit = cdrMillimeter;
         const r = ActiveLayer.CreateRectangle2(0, 0, 10, 10); r.Name = 'r';
         r.Fill.UniformColor.RGBAssign(255, 0, 0);
         r.Outline.Width = 0.5;
         r.Outline.Color.CMYKAssign(0, 100, 100, 0);
         const c = new Color(); c.CMYKAssign(10, 20, 30, 40);
         print(r.Fill.Type === cdrUniformFill, r.Fill.UniformColor.RGBRed, r.Outline.Width, c.CMYKMagenta, c.Type === cdrColorCMYK);
         print(r.Fill.UniformColor.HexValue);",
    );
    let r = all(&app)
        .into_iter()
        .find(|s| s.name.as_deref() == Some("r"))
        .expect("r");
    assert_eq!(r.fill, Fill::Solid(Color::rgb8(255, 0, 0)));
    let st = r.stroke.expect("outline");
    assert!(close(st.width, 0.5));
    assert!(matches!(st.color, Color::Cmyk { .. }));
    assert_eq!(out[0], "true 255 0.5 20 true");
    assert_eq!(out[1].to_lowercase(), "#ff0000");
}

#[test]
fn text_story_edits_the_text_object() {
    let mut app = App::headless();
    let out = run(
        &mut app,
        "ActiveDocument.Unit = cdrMillimeter;
         const t = ActiveLayer.CreateArtisticText(10, 10, 'Hello', 0, 0, 'Arial', 18);
         t.Name = 't';
         const st = t.Text.Story;
         st.Text = 'Hello world'; st.Bold = true; st.Size = 30; st.Alignment = cdrCenterAlignment;
         print(t.Type === cdrTextShape, st.Text, st.Size, st.Bold, st.Words.Count);
         print(t.Text.Replace('world', 'there'), t.Text.Story.Text);",
    );
    assert_eq!(out[0], "true Hello world 30 true 2");
    assert_eq!(out[1], "true Hello there");
    let t = all(&app)
        .into_iter()
        .find(|s| s.name.as_deref() == Some("t"))
        .expect("t");
    let ShapeKind::Text { spans, align, .. } = &t.kind else {
        panic!("text");
    };
    assert!(spans.iter().all(|s| s.bold && s.size_pt == 30.0));
    assert_eq!(*align, tracedraw_core::TextAlign::Center);
}

#[test]
fn shaping_grouping_and_new_pages_keep_ids_unique() {
    let mut app = App::headless();
    run(
        &mut app,
        "ActiveDocument.Unit = cdrMillimeter;
         const a = ActiveLayer.CreateRectangle2(0, 0, 20, 20);
         const b = ActiveLayer.CreateRectangle2(10, 10, 20, 20);
         const w = b.Weld(a); w.Name = 'weld';
         const c = ActiveLayer.CreateEllipse2(80, 80, 5);
         const d = c.Duplicate(10, 0);
         const g = ActiveDocument.CreateShapeRangeFromArray(c, d).Group();
         g.Name = 'G';
         const g2 = g.Duplicate(0, 20);
         ActiveDocument.AddPages(1);
         ActivePage.Shapes.Item('G').UngroupAll();
         const p2 = ActiveDocument.Pages.Item(2);
         p2.ActiveLayer.CreateRectangle2(0, 0, 5, 5).Name = 'second page';",
    );
    let shapes = all(&app);
    let weld = shapes
        .iter()
        .find(|s| s.name.as_deref() == Some("weld"))
        .expect("weld");
    assert!(matches!(weld.kind, ShapeKind::Path { .. }));
    // The welded originals are gone: weld, group copy, two loose ellipses.
    let page1 = &app.doc().pages[0];
    assert_eq!(
        page1.layers.iter().map(|l| l.shapes.len()).sum::<usize>(),
        4
    );
    let mut ids: Vec<u64> = shapes.iter().map(|s| s.id.raw()).collect();
    let n = ids.len();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), n, "ids repeat");
    assert!(shapes
        .iter()
        .any(|s| s.name.as_deref() == Some("second page")));
    // The app's next id comes after every id the macro used.
    let next = app.engine.new_shape_id().raw();
    assert!(ids.iter().all(|i| *i < next));
}

#[test]
fn community_macro_resize_the_selection() {
    let mut app = App::headless();
    run(
        &mut app,
        "ActiveDocument.Unit = cdrMillimeter;
         for (let i = 0; i < 3; i++) ActiveLayer.CreateEllipse2(20 + i * 30, 50, 5 + i).AddToSelection();",
    );
    run(
        &mut app,
        "ActiveDocument.Unit = cdrMillimeter;
         ActiveDocument.ReferencePoint = cdrCenter;
         for (const s of ActiveSelectionRange) s.SetSize(20, 10);",
    );
    for id in app.selection.clone() {
        let b = page_bounds(&app, id);
        assert!(close(b.width(), 20.0) && close(b.height(), 10.0), "{b:?}");
    }
    assert_eq!(app.selection.len(), 3);
}

#[test]
fn community_macro_number_the_objects() {
    let mut app = App::headless();
    run(
        &mut app,
        "ActiveDocument.Unit = cdrMillimeter;
         for (let i = 0; i < 4; i++) ActiveLayer.CreateRectangle2(10 + i * 30, 10, 20, 20);
         const shapes = ActiveLayer.Shapes.All();
         for (let i = 1; i <= shapes.Count; i++) {
           const s = shapes.Item(i);
           const t = ActiveLayer.CreateArtisticText(0, 0, String(i));
           t.CenterX = s.CenterX; t.CenterY = s.CenterY;
         }",
    );
    let texts: Vec<Shape> = all(&app)
        .into_iter()
        .filter(|s| matches!(s.kind, ShapeKind::Text { .. }))
        .collect();
    assert_eq!(texts.len(), 4);
    for (i, t) in texts.iter().enumerate() {
        let c = page_bounds(&app, t.id).center();
        assert!((c.x - (20.0 + i as f64 * 30.0)).abs() < 1e-6, "{c:?}");
        assert!((c.y - 20.0).abs() < 1e-6, "{c:?}");
    }
}

#[test]
fn community_macro_grid_layout() {
    let mut app = App::headless();
    run(
        &mut app,
        "ActiveDocument.Unit = cdrMillimeter;
         ActiveDocument.ReferencePoint = cdrTopLeft;
         for (let i = 0; i < 9; i++) ActiveLayer.CreateEllipse2(100, 100, 4 + i % 3).AddToSelection();
         const sr = ActiveSelectionRange;
         let i = 0;
         for (const s of sr) {
           s.SetPosition(10 + (i % 3) * 25, 200 - Math.floor(i / 3) * 25);
           i++;
         }",
    );
    let mut lefts: Vec<(i64, i64)> = app
        .selection
        .iter()
        .map(|id| {
            let b = page_bounds(&app, *id);
            (b.x0.round() as i64, b.y1.round() as i64)
        })
        .collect();
    lefts.sort();
    assert_eq!(lefts.len(), 9);
    assert_eq!(lefts[0], (10, 150));
    assert_eq!(lefts[8], (60, 200));
}

#[test]
fn community_macro_replace_a_colour_across_the_page() {
    let mut app = App::headless();
    run(
        &mut app,
        "ActiveDocument.Unit = cdrMillimeter;
         const a = ActiveLayer.CreateRectangle2(0, 0, 10, 10); a.Fill.UniformColor.RGBAssign(255, 0, 0);
         const b = ActiveLayer.CreateRectangle2(20, 0, 10, 10); b.Fill.UniformColor.RGBAssign(255, 0, 0);
         const c = ActiveLayer.CreateRectangle2(40, 0, 10, 10); c.Fill.UniformColor.RGBAssign(0, 255, 0);
         ActiveDocument.CreateShapeRangeFromArray(b, c).Group();",
    );
    run(
        &mut app,
        "const red = CreateRGBColor(255, 0, 0);
         const blue = CreateRGBColor(0, 0, 255);
         for (const s of ActivePage.Shapes.FindShapes('', 0, true)) {
           if (s.Fill.Type === cdrUniformFill && s.Fill.UniformColor.IsSame(red)) s.Fill.UniformColor.CopyAssign(blue);
         }",
    );
    let fills: Vec<Fill> = all(&app)
        .into_iter()
        .filter(|s| matches!(s.kind, ShapeKind::Rect { .. }))
        .map(|s| s.fill)
        .collect();
    assert_eq!(
        fills
            .iter()
            .filter(|f| **f == Fill::Solid(Color::rgb8(0, 0, 255)))
            .count(),
        2
    );
    assert_eq!(
        fills
            .iter()
            .filter(|f| **f == Fill::Solid(Color::rgb8(0, 255, 0)))
            .count(),
        1
    );
}

#[test]
fn community_macro_export_every_page_as_png() {
    let mut app = App::headless();
    let dir = std::env::temp_dir().join(format!("tracedraw-vg-export-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    let src = format!(
        "ActiveDocument.Unit = cdrMillimeter;
         ActiveLayer.CreateRectangle2(10, 10, 50, 50).Fill.UniformColor.RGBAssign(0, 0, 255);
         ActiveDocument.AddPages(2);
         for (const p of ActiveDocument.Pages) {{
           p.Activate();
           ActiveDocument.ExportBitmap({dir:?} + '/page' + p.Index + '.png', cdrPNG, cdrCurrentPage, cdrRGBColorImage, 100, 0, 72, 72).Finish();
         }}",
        dir = dir.display().to_string()
    );
    run(&mut app, &src);
    for i in 1..=3 {
        let bytes = std::fs::read(dir.join(format!("page{i}.png"))).expect("png written");
        assert_eq!(&bytes[..4], b"\x89PNG");
        let img = image::load_from_memory(&bytes).expect("png").to_rgba8();
        assert_eq!(img.width(), 100);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn nodes_segments_and_curve_building() {
    let mut app = App::headless();
    let out = run(
        &mut app,
        "ActiveDocument.Unit = cdrMillimeter;
         const crv = ActiveDocument.CreateCurve();
         const sp = crv.CreateSubPath(0, 0);
         sp.AppendLineSegment(30, 0); sp.AppendLineSegment(30, 40); sp.Closed = true;
         const s = ActiveLayer.CreateCurve(crv);
         print(s.Type === cdrCurveShape, s.Curve.Nodes.Count, s.Curve.Segments.Count, s.Curve.Closed, Math.round(s.Curve.Length));
         s.Curve.Nodes.Item(2).SetPosition(40, 0);
         print(s.Curve.Nodes.Item(2).PositionX);",
    );
    assert_eq!(out[0], "true 3 3 true 120");
    assert_eq!(out[1], "40");
}

#[test]
fn errors_stop_the_macro_and_keep_what_ran() {
    let mut app = App::headless();
    let err = run_vgcore(
        &mut app,
        "ActiveLayer.CreateRectangle2(0, 0, 1, 1).Name = 'kept'; ActivePage.Shapes.Item(5);",
        "Macro",
    )
    .expect_err("out of range");
    assert!(err.contains("out of range"), "{err}");
    assert!(all(&app).iter().any(|s| s.name.as_deref() == Some("kept")));
}

// ----- Members published macros use -----------------------------------------

fn png(w: u32, h: u32, rgb: [u8; 3]) -> Vec<u8> {
    let img = image::RgbaImage::from_pixel(w, h, image::Rgba([rgb[0], rgb[1], rgb[2], 255]));
    crate::bitmap_fx::encode(&img).expect("png")
}

/// A 20 x 10 pixel bitmap, one inch wide and half an inch high.
fn bitmap(app: &mut App) -> ShapeId {
    app.new_shape(ShapeKind::Bitmap {
        rect: Rect::new(0.0, 0.0, 25.4, 12.7),
        width_px: 20,
        height_px: 10,
        png: png(20, 10, [200, 30, 30]).into(),
        fx: None,
    })
    .expect("bitmap")
}

#[test]
fn progress_shows_in_the_status_line() {
    let mut app = App::headless();
    run(
        &mut app,
        "Application.Status.BeginProgress('Converting', false);
         for (let i = 1; i <= 4; i++) Application.Status.Progress = i * 10;",
    );
    assert_eq!(app.status, "Converting 40%");
    run(
        &mut app,
        "Status.BeginProgress('Again'); Status.EndProgress();",
    );
    assert_eq!(app.status, "");
}

#[test]
fn bitmaps_report_resolution_and_mode_and_resample() {
    let mut app = App::headless();
    let id = bitmap(&mut app);
    let out = run(
        &mut app,
        &format!(
            "const b = ActivePage.Shapes.FindShape('', 0, {id}).Bitmap;
             print(b.ResolutionX, b.ResolutionY, b.Mode === cdrRGBColorImage, b.Transparent, b.ExternallyLinked);
             b.Resample(0, 0, true, 40, 40);
             b.ConvertTo(cdrGrayscaleImage);
             print(b.SizeWidth, b.SizeHeight, b.ResolutionX, b.Mode === cdrGrayscaleImage);",
            id = id.raw()
        ),
    );
    assert_eq!(out[0], "20 20 true false false");
    assert_eq!(out[1], "40 20 40 true");
}

#[test]
fn objects_convert_to_bitmaps() {
    let mut app = App::headless();
    let out = run(
        &mut app,
        "ActiveDocument.Unit = cdrInch;
         const r = ActiveLayer.CreateRectangle2(1, 1, 2, 1);
         r.Fill.UniformColor.RGBAssign(0, 0, 255);
         const b = r.ConvertToBitmapEx(cdrGrayscaleImage, false, false, 72, cdrNormalAntiAliasing);
         print(b.Type === cdrBitmapShape, b.Bitmap.ResolutionX, b.Bitmap.Mode === cdrGrayscaleImage, ActivePage.Shapes.Count);",
    );
    assert_eq!(out[0], "true 72 true 1");
}

#[test]
fn power_clip_contents_and_parent() {
    let mut app = App::headless();
    let out = run(
        &mut app,
        "ActiveDocument.Unit = cdrMillimeter;
         const frame = ActiveLayer.CreateRectangle2(0, 0, 50, 50); frame.Name = 'frame';
         const inside = ActiveLayer.CreateEllipse2(25, 25, 40); inside.Name = 'inside';
         inside.AddToClipFrame(frame);
         const f = ActivePage.Shapes.Item('frame');
         f.ClipFrame.EnterEditMode(); f.ClipFrame.LeaveEditMode();
         print(f.ClipFrame.Shapes.Count, f.ClipFrame.Shapes.Item(1).Name, f.ClipFrame.Shapes.Item(1).ClipFrameParent.Name);
         print(ActivePage.Shapes.Count, ActiveLayer.CreateRectangle2(0, 0, 1, 1).ClipFrame === null);",
    );
    assert_eq!(out[0], "1 inside frame");
    assert_eq!(out[1], "1 true");
}

#[test]
fn effects_list_drop_shadows_and_separate_them() {
    let mut app = App::headless();
    let id = app
        .new_shape(ShapeKind::Rect {
            rect: Rect::new(0.0, 0.0, 20.0, 20.0),
            radius: 0.0,
            corners: None,
        })
        .expect("rect");
    app.run(tracedraw_core::Command::SetShadow {
        shapes: vec![id],
        shadow: Some(tracedraw_core::Shadow::default()),
    });
    let out = run(
        &mut app,
        &format!(
            "const s = ActivePage.Shapes.FindShape('', 0, {id});
             const e = s.Effects.Item(1);
             print(s.Effects.Count, e.Type === cdrDropShadow, e.DropShadow.MergeMode === cdrMergeNormal);
             let msg = '';
             try {{ e.DropShadow.MergeMode = cdrMergeMultiply; }} catch (x) {{ msg = String(x); }}
             print(msg.includes('DropShadow.MergeMode'));
             print(e.Separate().Count >= 2);",
            id = id.raw()
        ),
    );
    assert_eq!(out[0], "1 true true");
    assert_eq!(out[1], "true");
    assert_eq!(out[2], "true");
}

#[test]
fn guides_are_created_and_turned() {
    let mut app = App::headless();
    let out = run(
        &mut app,
        "ActiveDocument.Unit = cdrMillimeter;
         const g = ActiveLayer.CreateGuide(0, 50, 100, 50);
         print(g.Type === cdrGuidelineShape, g.Guide.Angle, g.Guide.PointY);
         g.Guide.SetPointAndAngle(30, 0, 90);
         print(ActivePage.Guides.Count, ActivePage.Guides.Item(1).Guide.Angle, ActivePage.Guides.Item(1).Guide.PointX);",
    );
    assert_eq!(out[0], "true 0 50");
    assert_eq!(out[1], "1 90 30");
    let guides = &app.doc().pages[0].guides;
    assert!(matches!(
        guides[0].line,
        tracedraw_core::document::GuideLine::Vertical { x } if (x - 30.0).abs() < 1e-9
    ));
}

#[test]
fn fountain_colours_and_transparency() {
    let mut app = App::headless();
    let out = run(
        &mut app,
        "const r = ActiveLayer.CreateRectangle2(0, 0, 1, 1);
         r.Fill.ApplyFountainFill(CreateRGBColor(255, 0, 0), CreateRGBColor(0, 0, 255), cdrLinearFountainFill, 0);
         const f = r.Fill.Fountain;
         print(r.Fill.Type === cdrFountainFill, f.Colors.Count, f.StartColor.RGBRed, f.EndColor.RGBBlue);
         f.Colors.Item(2).Color.RGBAssign(0, 255, 0);
         let steps = '';
         try { f.Steps = 10; } catch (x) { steps = String(x); }
         r.Transparency.ApplyUniformTransparency(30);
         print(r.Fill.Fountain.EndColor.RGBGreen, steps.includes('Fill.Fountain.Steps'), r.Transparency.Type === cdrUniformTransparency, r.Transparency.Uniform);",
    );
    assert_eq!(out[0], "true 2 255 255");
    assert_eq!(out[1], "255 true true 30");
}

#[test]
fn text_words_ranges_overflow_and_spacing() {
    let mut app = App::headless();
    let out = run(
        &mut app,
        "const t = ActiveLayer.CreateArtisticText(0, 0, 'one two three', 0, 0, 'Arial', 10);
         const st = t.Text.Story;
         print(st.Words.Count, st.Words.Item(2).Text, st.Characters.Count, t.Text.Overflow);
         st.Range(4, 7).Size = 20;
         st.LineSpacing = 150;
         let sizes = [];
         for (const w of st.Words) sizes.push(w.Size);
         print(sizes.join(','), st.Text, st.LineSpacing);",
    );
    assert_eq!(out[0], "3 two 13 false");
    assert_eq!(out[1], "10,20,10 one two three 150");
}

#[test]
fn curve_areas_and_the_shape_tool_selection() {
    let mut app = App::headless();
    let id = app
        .new_shape(ShapeKind::Rect {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            radius: 0.0,
            corners: None,
        })
        .expect("rect");
    run(
        &mut app,
        &format!(
            "ActivePage.Shapes.FindShape('', 0, {}).ConvertToCurves();",
            id.raw()
        ),
    );
    app.node_selection = vec![(id, 0), (id, 2)];
    let out = run(
        &mut app,
        &format!(
            "ActiveDocument.Unit = cdrCentimeter;
             const c = ActivePage.Shapes.FindShape('', 0, {}).Curve;
             const nr = c.Selection();
             print(Math.round(c.Area * 1000) / 1000, Math.round(c.SubPaths.Item(1).Area * 1000) / 1000, nr.Count, nr.LastNode.PositionX, c.Nodes.FirstNode.PositionX);",
            id.raw()
        ),
    );
    assert_eq!(out[0], "1 1 2 1 0");
}

#[test]
fn documents_pages_layers_and_settings() {
    let mut app = App::headless();
    let id = app
        .new_shape(ShapeKind::Rect {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            radius: 0.0,
            corners: None,
        })
        .expect("rect");
    app.run(tracedraw_core::Command::SetLocked {
        shapes: vec![id],
        locked: true,
    });
    let out = run(
        &mut app,
        "ActiveDocument.SaveSettings();
         ActiveDocument.Unit = cdrMillimeter; ActiveDocument.ReferencePoint = cdrTopLeft;
         ActiveDocument.RestoreSettings();
         const l = ActiveLayer;
         print(ActiveDocument.Unit === cdrInch, ActiveDocument.ReferencePoint === cdrCenter, l.IsGuidesLayer, l.IsGridLayer, l.IsSpecialLayer);
         ActivePage.UnlockAllShapes();
         const s = ActivePage.Shapes.Item(1);
         print(s.Locked, s.CanHaveFill, s.CanHaveOutline, typeof ActiveWindow.ActiveView.OriginX, ActiveDocument.Rulers.HUnits === cdrMillimeter);
         print(ActiveDocument.MasterPage.Layers.Count >= 0, ColorManager.DefaultColorContext.RGBColorProfile.Name.length > 0, ActiveDocument.PreserveSelection);",
    );
    assert_eq!(out[0], "true true false false false");
    assert_eq!(out[1], "false true true number true");
    assert_eq!(out[2], "true true true");
    assert!(!app.doc().find_shape(id).expect("rect").locked);
}

#[test]
fn members_without_a_counterpart_raise_named_errors() {
    for (src, member) in [
        ("ActiveLayer.Paste();", "Layer.Paste"),
        (
            "Application.FrameWork.ShowDocker('x');",
            "Application.FrameWork",
        ),
        ("ActiveDocument.GetUserClick();", "Document.GetUserClick"),
        (
            "ActiveLayer.CreateArtisticText(0, 0, 'a').Text.Story.LanguageID;",
            "Story.LanguageID",
        ),
        (
            "CreateDocumentEx(CreateStructCreateOptions());",
            "Application.CreateDocumentEx",
        ),
    ] {
        let mut app = App::headless();
        let err = run_vgcore(&mut app, src, "Macro").expect_err(member);
        assert!(err.contains(member), "{src}: {err}");
        assert!(err.contains("not available in TraceDraw"), "{err}");
    }
}

#[test]
fn constants_macros_use_exist_with_their_values() {
    let mut app = App::headless();
    let out = run(
        &mut app,
        "const names = ['cdrNoTransparency', 'cdrUniformTransparency', 'cdrFountainTransparency',
           'cdrMergeNormal', 'cdrMergeMultiply', 'cdrDropShadow', 'cdrContour', 'cdrBlend', 'cdrLens',
           'cdrEnvelope', 'cdrPerspective', 'cdrDistortion', 'cdrTextOnPath', 'cdrControlPath',
           'cdrLensMagnify', 'cdrLensFishEye', 'cdrLensInvert', 'cdrPixelView', 'cdrEnhancedView',
           'cdrSimpleWireframeView', 'cdrNoAntiAliasing', 'cdrNormalAntiAliasing', 'cdrLABImage',
           'cdrDuotoneImage', 'cdrCMYKMultiChannelImage', 'cdrSmallCapsFontCase', 'cdrAllCapsFontCase',
           'cdrEnglishUS', 'cdrEnglishUK', 'cdrRussian', 'cdrOutlineRoundLineJoin', 'cdrEnhancedOutline',
           'cdrImportFull', 'cdrCursorEyeDrop', 'cdrCurrentVersion', 'cdrCompressionLZW', 'cdrCPT',
           'cdrApplyToFill', 'cdrApplyToFillAndOutline', 'cdrColorPantoneHex', 'cdrColorUserInk',
           'cdrColorMultiChannel', 'cdrLinkAllConnections'];
         print(names.filter(n => typeof globalThis[n] !== 'number').join(','));
         print(cdrInch, cdrMillimeter, cdrCenter, cdrBottomLeft, cdrCurveShape, cdrUniformFill, cdrColorCMYK, cdrEnglishUS);
         print(cdrTextureFill, cdrPatternFill, cdrHatchFill, cdrCDR, cdrTIFF, cdrPDF, cdrCPT, cdrWEBP);
         print(cdrMergeMultiply, cdrMergeDifference, cdrBlend, cdrDropShadow, cdrInnerShadow, cdrApplyToFill,
           cdrCursorPick, cdrColorUserInk, cdrUnitQ, cdrAlignDistributeHAlignLeft, cdrDistributeToRect,
           cdrMixedAlignment, cdrUndefined, cdrOutlineUndefinedLineCaps);
         print(typeof cdrBevel, typeof cdrUISUnit);",
    );
    assert_eq!(out[0], "");
    assert_eq!(out[1], "1 3 9 5 3 1 2 1033");
    assert_eq!(out[2], "8 9 10 1795 772 1333 1808 825");
    assert_eq!(out[3], "9 24 0 5 11 0 305 22 15 2 2 6 -2 -1");
    assert_eq!(out[4], "undefined undefined");
}

#[test]
fn align_and_distribute_take_the_reference_methods() {
    let mut app = App::headless();
    let out = run(
        &mut app,
        "ActiveDocument.Unit = cdrMillimeter;
         const l = ActiveLayer;
         const a = l.CreateRectangle2(10, 10, 10, 10), b = l.CreateRectangle2(40, 22, 20, 10),
           c = l.CreateRectangle2(100, 50, 10, 30);
         const r = ActiveDocument.CreateShapeRangeFromArray(a, b, c);
         r.AlignAndDistribute(cdrAlignDistributeHAlignLeft, cdrAlignDistributeVNone);
         print(a.LeftX, b.LeftX, c.LeftX, a.BottomY, b.BottomY);
         r.AlignAndDistribute(cdrAlignDistributeHNone, cdrAlignDistributeVDistributeSpacing);
         print(a.BottomY, a.TopY, b.BottomY, b.TopY, c.BottomY);
         r.AlignAndDistribute(cdrAlignDistributeHAlignCenter, 0, cdrAlignShapesToCenterOfPage);
         print(Math.round(a.CenterX * 1000) / 1000 === Math.round(ActivePage.SizeWidth / 2 * 1000) / 1000, b.CenterX === a.CenterX);
         r.AlignAndDistribute(cdrAlignDistributeHAlignRight, 0, cdrAlignShapesToPoint, 0, false, 0, 150, 0);
         print(a.RightX, c.RightX);
         try { r.AlignAndDistribute(cdrAlignDistributeHAlignLeft, 0, cdrAlignShapesToGrid); } catch (x) { print(String(x).includes('AlignAndDistribute')); }",
    );
    assert_eq!(out[0], "100 100 100 10 22");
    // Gaps of (80 - 10 - 10 - 10 - 30) / 2 = 10 between the rectangles.
    assert_eq!(out[1], "10 20 30 40 50");
    assert_eq!(out[2], "true true");
    assert_eq!(out[3], "150 150");
    assert_eq!(out[4], "true");
}
