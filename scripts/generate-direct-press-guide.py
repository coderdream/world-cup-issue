from pathlib import Path

from PIL import Image, ImageDraw


OUTPUT = Path(r"D:\AI\apps\ComfyUI\input\ui_tests\direct_press_detailed_guide.png")
W, H = 1536, 864
INK = (35, 35, 35)
LIGHT = (125, 125, 125)
ORANGE = (235, 130, 30)


def line(draw, points, width=3, color=INK):
    draw.line(points, fill=color, width=width, joint="curve")


def main():
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    image = Image.new("RGB", (W, H), "white")
    draw = ImageDraw.Draw(image)

    # A thin, imperfect paper ball with internal folds.
    paper = [(135, 510), (180, 435), (255, 420), (315, 365), (385, 402),
             (450, 380), (515, 445), (480, 525), (420, 545), (355, 585),
             (270, 565), (195, 550)]
    line(draw, paper, 4)
    for folds in [[(180, 475), (250, 455), (315, 500), (385, 430)],
                  [(220, 530), (280, 470), (350, 550), (430, 440)],
                  [(300, 420), (320, 480), (360, 520)],
                  [(385, 402), (370, 455), (450, 485)]]:
        line(draw, folds, 2, LIGHT)

    # Press body, feet, top plate, threaded screw, and inner platen.
    draw.rounded_rectangle((560, 275, 1030, 735), radius=28, outline=INK, width=5)
    draw.rounded_rectangle((566, 281, 1024, 729), radius=27, outline=LIGHT, width=1)
    for x in (615, 950):
        draw.ellipse((x, 720, x + 42, 747), outline=INK, width=3)
        line(draw, [(x + 12, 740), (x + 5, 770)], 3)
    draw.rounded_rectangle((650, 220, 945, 285), radius=28, outline=INK, width=5)
    draw.ellipse((790, 245, 812, 258), outline=INK, width=2)
    line(draw, [(795, 285), (795, 395)], 4)
    for y in range(300, 390, 14):
        line(draw, [(775, y), (815, y)], 2)
    draw.rounded_rectangle((635, 395, 955, 590), radius=10, outline=INK, width=4)
    draw.rectangle((660, 425, 930, 555), outline=LIGHT, width=2)
    line(draw, [(685, 470), (900, 470)], 2, LIGHT)
    line(draw, [(685, 495), (900, 495)], 4)
    line(draw, [(685, 520), (900, 520)], 2, LIGHT)
    for x, y in ((650, 420), (940, 420), (650, 565), (940, 565)):
        draw.ellipse((x - 7, y - 7, x + 7, y + 7), outline=INK, width=2)

    # Side handle and orange force accent.
    draw.ellipse((550, 450, 630, 530), outline=INK, width=4)
    line(draw, [(590, 490), (470, 560)], 5)
    draw.ellipse((435, 545, 490, 600), outline=INK, width=4)
    line(draw, [(590, 490), (470, 560)], 2, ORANGE)

    # Key output and an inviting hand gesture.
    line(draw, [(1030, 560), (1180, 620)], 5)
    line(draw, [(1040, 600), (1180, 650)], 4)
    draw.ellipse((1160, 600, 1220, 660), outline=INK, width=4)
    line(draw, [(1190, 650), (1190, 735)], 4)
    line(draw, [(1280, 610), (1390, 580), (1470, 610)], 4)
    line(draw, [(1300, 640), (1390, 625), (1470, 650)], 4)

    # Small black character and stool, with active arms.
    draw.ellipse((350, 590, 470, 750), fill=INK)
    draw.ellipse((385, 625, 402, 642), fill="white")
    draw.ellipse((430, 625, 447, 642), fill="white")
    line(draw, [(380, 740), (365, 820)], 4)
    line(draw, [(440, 740), (455, 820)], 4)
    line(draw, [(390, 700), (335, 650), (470, 555)], 4)
    line(draw, [(435, 700), (500, 660)], 4)
    draw.rectangle((320, 805, 500, 830), outline=INK, width=4)
    line(draw, [(335, 830), (315, 860)], 3)
    line(draw, [(480, 830), (500, 860)], 3)

    image.save(OUTPUT)
    print(OUTPUT)


if __name__ == "__main__":
    main()
