# Page is at x 422..940, y 132..862 at 65% (from the earlier screenshot).
click(790, 345)          # select ellipse
time.sleep(0.3); shot("t1_select")
click(790, 345)          # second click: rotate mode
time.sleep(0.3); shot("t1_rotate")
click(600, 500)          # deselect on empty page
key("F6")                # rectangle tool
drag(500, 760, 700, 840) # draw rectangle
click(1320, 356, 1)      # palette: left click a colour (fill)
click(1320, 120, 3)      # palette: right click (outline)
time.sleep(0.3); shot("t1_rect")
key("F8")                # text tool
click(700, 700)
typestr("Hello")
key("Escape")
time.sleep(0.3); shot("t1_text")
