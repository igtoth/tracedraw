"""Drive the app under Xvfb with synthetic pointer/keyboard events and take screenshots."""
import os, sys, time, subprocess
from Xlib import display, X
from Xlib.ext import xtest

os.environ["DISPLAY"] = ":99"
d = display.Display(":99")

def move(x, y):
    d.screen().root.warp_pointer(x, y); d.sync(); time.sleep(0.05)

def click(x, y, button=1):
    move(x, y)
    xtest.fake_input(d, X.ButtonPress, button); d.sync(); time.sleep(0.06)
    xtest.fake_input(d, X.ButtonRelease, button); d.sync(); time.sleep(0.25)

def drag(x0, y0, x1, y1, button=1, steps=12):
    move(x0, y0)
    xtest.fake_input(d, X.ButtonPress, button); d.sync(); time.sleep(0.1)
    for i in range(1, steps + 1):
        move(x0 + (x1 - x0) * i // steps, y0 + (y1 - y0) * i // steps); time.sleep(0.03)
    time.sleep(0.1)
    xtest.fake_input(d, X.ButtonRelease, button); d.sync(); time.sleep(0.3)

def key(keysym_name):
    from Xlib import XK
    ks = XK.string_to_keysym(keysym_name)
    kc = d.keysym_to_keycode(ks)
    xtest.fake_input(d, X.KeyPress, kc); d.sync(); time.sleep(0.04)
    xtest.fake_input(d, X.KeyRelease, kc); d.sync(); time.sleep(0.15)

def typestr(s):
    for ch in s:
        name = {" ": "space"}.get(ch, ch)
        key(name)

def shot(name):
    subprocess.run(["import", "-display", ":99", "-window", "root", f"/tmp/{name}.png"], check=True)
    print("shot", name)

def focus():
    root = d.screen().root
    for w in root.query_tree().children:
        try:
            attrs = w.get_attributes()
            if attrs.map_state == X.IsViewable and w.get_geometry().width > 500:
                d.set_input_focus(w, X.RevertToParent, X.CurrentTime); d.sync()
                return
        except Exception:
            pass

if __name__ == "__main__":
    focus()
    script = sys.argv[1]
    exec(open(script).read())
