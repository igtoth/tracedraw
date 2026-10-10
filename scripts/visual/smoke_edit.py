# Draw one of everything, then run the editing commands a user runs most,
# checking after each that the application is still alive. Run with
# scripts/visual/run.sh scripts/visual/smoke_edit.py
def check(tag):
    print(tag, "alive" if alive() else "DEAD")
click(800, 600); combo("Control_L", "n"); time.sleep(0.8)   # Welcome: File > New
key("Return"); time.sleep(0.8)                        # Create a New Document: OK
for k, (x0, y0, x1, y1) in [("F6", (450, 200, 550, 280)), ("F7", (600, 200, 700, 280)),
                            ("y", (750, 200, 850, 280)), ("a", (450, 320, 550, 400)),
                            ("d", (600, 320, 700, 400))]:
    key(k); drag(x0, y0, x1, y1); time.sleep(0.2)
key("F5"); drag(450, 450, 600, 520); time.sleep(0.2)
key("F8"); click(470, 600); typestr("Smoke test"); key("Escape"); time.sleep(0.2)
check("drawn")
combo("Control_L", "a"); click(546, 893); check("palette")
combo("Control_L", "z"); combo("Control_L", "Shift_L", "z"); check("undo redo")
key("F10"); click(800, 240); drag(850, 240, 870, 260); check("node drag")
key("space"); click(500, 240); click(500, 240, button=3); time.sleep(0.4); key("Escape"); check("context menu")
combo("Control_L", "a"); combo("Control_L", "g"); combo("Control_L", "u"); check("group ungroup")
combo("Control_L", "l"); combo("Control_L", "k"); combo("Control_L", "q"); check("combine break convert")
click(17, 505); drag(500, 240, 540, 270); check("drop shadow")
click(17, 540); drag(650, 240, 620, 220); check("transparency")
key("F2"); click(600, 400); key("F4"); key("h"); drag(600, 400, 650, 420); key("F4"); check("zoom pan")
click(175, 872); click(77, 872); check("pages")
for y in (105, 180, 250, 340, 430):
    click(1583, y); time.sleep(0.3)
check("dockers")
shot("smoke_edit_end")
