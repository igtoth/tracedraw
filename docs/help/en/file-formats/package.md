# Packaging a drawing

A drawing that links to picture files and uses fonts other computers may not
have does not travel well as a single file. A package puts the drawing and
everything it needs into one folder: the drawing, the pictures it links to,
the font files its text uses, a report and, if you want, a PDF. Hand the
folder to a print shop or a colleague, or keep it with the finished job.

> **Note:** Packages need the desktop version. In the browser version the
> menu command is greyed out and Save As does not offer the choice.

## Making a package {#make}

1. Choose [[menu.file > menu.file.package]].
2. In the [[package.title]] dialog box, check the [[package.folder]]. It is
   a new folder named after the drawing, in the folder of the drawing's file
   (else the folder you used last, else your home folder).
   [[publish.browse]] picks another place for it and keeps the folder's
   name.
3. Check the [[dialog.file_name]] of the drawing in the package (the
   drawing's name, as a `.tdraw` file).
4. Tick [[package.pdf]] to add a PDF of the drawing (off at first).
5. Click [[dialog.ok]].

TraceDraw makes the folder when it does not exist and replaces files of the
same names in it. The drawing you have open does not change: its links still
point to the original pictures. The status bar says how many pictures and
fonts went in, and how many items could not (the report names them).

## Saving as a package {#save-as}

You can also package a drawing while you save it:

1. Choose [[menu.file > menu.file.save_as]].
2. Choose TraceDraw (`*.tdraw`) as the [[dialog.save_as_type]].
3. Tick [[dialog.save_package]] and, if you want a PDF too, [[package.pdf]].
4. Type the name and click [[dialog.save]].

The drawing is saved as `<name>/<name>.tdraw` in the folder you chose, with
its pictures copied beside it. Unlike [[menu.file.package]], Save As changes
the open drawing: its links now point to the copies in the package, so from
then on you work on the package's drawing.

## What goes into the folder {#contents}

| Item | Where | What it holds |
|---|---|---|
| The drawing | `<name>.tdraw` | The drawing, with the fonts it already carries. |
| Linked pictures | `Pictures` folder | Every picture the drawing links to, in groups, clip frames and symbols too, each file once. Two files of the same name are kept apart as `logo.png` and `logo-2.png`. |
| Fonts | `Fonts` folder | One font file for each face the text uses, but only fonts whose licence allows them to go with a document. |
| A PDF (optional) | `<name>.pdf` | Written with the choices you made last in Publish to PDF, with the drawing's bleed. |
| A report | `<name> report.txt` | What went in, and what could not: pictures not found, fonts left out because of their licence, and fonts that are not installed with the font that draws their text. |

The links of the drawing in the package point to the copies as paths relative
to the drawing (`Pictures/logo.png`), so you can move, copy or send the whole
folder and the links keep working. A picture whose file is gone keeps its old
link and is named in the report; its pixels are in the drawing anyway, so it
still shows.

## Opening a package {#open}

Open the `.tdraw` file of the package as any drawing. When the drawing's
folder has a `Fonts` folder beside it, TraceDraw uses the fonts in it for
this session, so the text shows in its own fonts even where they are not
installed. Nothing is installed on the computer.

> **Tip:** Links relative to a drawing are read from the drawing's folder
> everywhere, also when a linked picture is updated from its file, so a
> package can live anywhere.

To check the drawing before packaging it (missing fonts, pictures at low
resolution, colours), run the [preflight](../printing/preflight.md) first.
