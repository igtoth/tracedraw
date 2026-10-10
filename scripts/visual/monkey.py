# Random clicks, drags, keys and chords over the whole window; fails
# when the application dies. MONKEY_SEED and MONKEY_STEPS choose the run.
import os, random
random.seed(int(os.environ.get("MONKEY_SEED", "1")))
steps = int(os.environ.get("MONKEY_STEPS", "150"))
click(800, 600); combo("Control_L", "n"); time.sleep(0.8); key("Return"); time.sleep(0.8)
key("F6"); drag(450, 250, 600, 350); key("F7"); drag(650, 250, 800, 350)
keys = ["F6", "F7", "F8", "F5", "F10", "space", "y", "a", "d", "g", "x", "i", "s", "Escape",
        "Delete", "Return", "Tab", "Left", "Right", "Up", "Down", "F2", "F4", "z", "h"]
regions = [(40, 100, 1250, 860), (0, 80, 36, 620), (0, 50, 1300, 75), (1270, 80, 1600, 880),
           (40, 880, 1300, 900), (0, 20, 600, 45)]
log = []
for i in range(steps):
    r = random.random()
    if r < 0.45:
        reg = random.choice(regions)
        x, y = random.randint(reg[0], reg[2]), random.randint(reg[1], reg[3])
        click(x, y); log.append(f"click {x},{y}")
    elif r < 0.7:
        a = [random.randint(300, 1000), random.randint(150, 800), random.randint(300, 1000), random.randint(150, 800)]
        drag(*a, steps=6); log.append(f"drag {a}")
    elif r < 0.8:
        x, y = random.randint(300, 1000), random.randint(150, 800)
        click(x, y, button=3); time.sleep(0.2); key("Escape"); log.append(f"rclick {x},{y}")
    elif r < 0.95:
        k = random.choice(keys); key(k); log.append(f"key {k}")
    else:
        k = random.choice(["a", "d", "g", "l", "k", "q", "u", "z", "c", "v", "x", "j", "y"])
        combo("Control_L", k); log.append(f"ctrl+{k}")
    if i % 10 == 9 and not alive():
        print("DEAD after:", log[-12:])
        break
else:
    print("survived", len(log), "actions")
