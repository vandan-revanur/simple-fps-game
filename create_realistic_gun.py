#!/usr/bin/env python3
"""Create a high-quality, realistic FPS gun sprite"""

from PIL import Image, ImageDraw, ImageFilter
import math

# Higher resolution for quality
width, height = 2048, 1024
img = Image.new('RGBA', (width, height), (0, 0, 0, 0))
draw = ImageDraw.Draw(img)

# Realistic metal colors with more variation
base_metal = (65, 68, 75)
dark_metal = (35, 38, 42)
highlight_metal = (120, 125, 135)
bright_highlight = (180, 185, 195)
shadow = (18, 20, 22)
barrel_dark = (25, 27, 30)
worn_metal = (85, 88, 95)

# Polymer/plastic for grips
polymer_base = (25, 25, 28)
polymer_highlight = (45, 45, 48)

def add_alpha(color, alpha):
    return color[:3] + (alpha,)

# Modern tactical rifle/carbine
# Work from muzzle (right) to stock (left)

# === BARREL AND MUZZLE ===
barrel_y = 380
barrel_height = 80
barrel_start = 800
barrel_end = 1800

# Main barrel cylinder
for i in range(barrel_height):
    shade = int(35 + 40 * math.sin(math.pi * i / barrel_height))
    color = (shade, shade + 3, shade + 8, 255)
    draw.line([(barrel_start, barrel_y + i), (barrel_end - 40, barrel_y + i)],
              fill=color, width=1)

# Barrel top highlight
draw.line([(barrel_start, barrel_y + 8), (barrel_end - 40, barrel_y + 8)],
          fill=add_alpha(bright_highlight, 200), width=3)

# Barrel bottom shadow
draw.line([(barrel_start, barrel_y + barrel_height - 5),
           (barrel_end - 40, barrel_y + barrel_height - 5)],
          fill=add_alpha(shadow, 200), width=3)

# Muzzle device (flash hider)
muzzle_x = barrel_end - 50
for slot in range(4):
    y = barrel_y + 15 + slot * 18
    # Slots in flash hider
    draw.rectangle([muzzle_x, y, barrel_end + 20, y + 12],
                   fill=add_alpha(dark_metal, 255), outline=add_alpha(shadow, 255))
    # Highlights
    draw.line([(muzzle_x, y + 2), (barrel_end + 15, y + 2)],
              fill=add_alpha(highlight_metal, 150), width=1)

# Muzzle opening
draw.ellipse([barrel_end + 5, barrel_y + 25, barrel_end + 25, barrel_y + 55],
             fill=add_alpha((10, 10, 12), 255), outline=add_alpha(shadow, 255))

# === UPPER RECEIVER AND RAIL ===
rail_y = barrel_y - 60
rail_height = 50
receiver_start = 300
receiver_end = barrel_start + 150

# Main upper receiver body with gradient
for i in range(rail_height):
    shade = int(60 + 25 * math.sin(math.pi * i / rail_height))
    color = (shade, shade + 3, shade + 8, 255)
    draw.line([(receiver_start, rail_y + i), (receiver_end, rail_y + i)],
              fill=color, width=1)

# Picatinny rail slots (top)
for x in range(receiver_start + 40, receiver_end - 40, 35):
    # Slot cutout
    draw.rectangle([x, rail_y + 5, x + 22, rail_y + 18],
                   fill=add_alpha(dark_metal, 255))
    draw.rectangle([x, rail_y + 32, x + 22, rail_y + 45],
                   fill=add_alpha(dark_metal, 255))
    # Raised section
    draw.rectangle([x + 2, rail_y + 19, x + 20, rail_y + 31],
                   fill=add_alpha(highlight_metal, 255))

# Front sight assembly
front_sight_x = receiver_end - 100
draw.rectangle([front_sight_x, rail_y - 25, front_sight_x + 30, rail_y + rail_height],
               fill=add_alpha(base_metal, 255), outline=add_alpha(shadow, 255))
# Front sight post
draw.polygon([(front_sight_x + 10, rail_y - 20),
              (front_sight_x + 20, rail_y - 20),
              (front_sight_x + 15, rail_y - 35)],
             fill=add_alpha(bright_highlight, 255), outline=add_alpha(shadow, 255))

# Rear sight
rear_sight_x = receiver_start + 150
draw.rectangle([rear_sight_x, rail_y - 30, rear_sight_x + 35, rail_y + rail_height],
               fill=add_alpha(base_metal, 255), outline=add_alpha(shadow, 255))
# Rear aperture
draw.rectangle([rear_sight_x + 5, rail_y - 15, rear_sight_x + 10, rail_y - 5],
               fill=add_alpha(highlight_metal, 255))
draw.rectangle([rear_sight_x + 25, rail_y - 15, rear_sight_x + 30, rail_y - 5],
               fill=add_alpha(highlight_metal, 255))
draw.ellipse([rear_sight_x + 12, rail_y - 13, rear_sight_x + 23, rail_y - 7],
             fill=add_alpha((0, 0, 0), 255))

# === LOWER RECEIVER ===
lower_top = rail_y + rail_height
lower_bottom = barrel_y + barrel_height + 20
lower_start = receiver_start
lower_end = barrel_start

# Main lower receiver with gradient
for i in range(int(lower_bottom - lower_top)):
    shade = int(55 + 20 * math.sin(math.pi * i / (lower_bottom - lower_top)))
    color = (shade, shade + 3, shade + 8, 255)
    draw.line([(lower_start, lower_top + i), (lower_end, lower_top + i)],
              fill=color, width=1)

# === MAGAZINE WELL AND MAGAZINE ===
mag_well_x = receiver_start + 350
mag_well_width = 120

# Magazine well opening
draw.polygon([
    (mag_well_x, lower_bottom - 5),
    (mag_well_x + mag_well_width, lower_bottom - 5),
    (mag_well_x + mag_well_width - 10, lower_bottom + 250),
    (mag_well_x + 10, lower_bottom + 250)
], fill=add_alpha(dark_metal, 255), outline=add_alpha(shadow, 255))

# Magazine body
mag_body_color = (40, 38, 35, 255)
draw.rectangle([mag_well_x + 8, lower_bottom,
                mag_well_x + mag_well_width - 8, lower_bottom + 240],
               fill=mag_body_color, outline=add_alpha(shadow, 255))

# Magazine ribbing
for y in range(int(lower_bottom + 20), int(lower_bottom + 230), 20):
    draw.line([(mag_well_x + 12, y), (mag_well_x + mag_well_width - 12, y)],
              fill=add_alpha(polymer_base, 200), width=2)

# Magazine base plate
draw.rectangle([mag_well_x + 5, lower_bottom + 235,
                mag_well_x + mag_well_width - 5, lower_bottom + 250],
               fill=add_alpha(dark_metal, 255), outline=add_alpha(shadow, 255))

# === TRIGGER GROUP ===
trigger_x = mag_well_x + 180
trigger_guard_y = lower_bottom - 10

# Trigger guard
guard_points = [
    (trigger_x, trigger_guard_y),
    (trigger_x + 100, trigger_guard_y),
    (trigger_x + 90, trigger_guard_y + 70),
    (trigger_x + 10, trigger_guard_y + 70)
]
draw.polygon(guard_points, outline=add_alpha(dark_metal, 255), width=6)

# Trigger
trigger_color = (90, 85, 80, 255)
draw.polygon([
    (trigger_x + 40, trigger_guard_y + 25),
    (trigger_x + 40, trigger_guard_y + 50),
    (trigger_x + 55, trigger_guard_y + 48),
    (trigger_x + 55, trigger_guard_y + 28)
], fill=trigger_color, outline=add_alpha(shadow, 255))

# === PISTOL GRIP ===
grip_x = lower_start + 100
grip_bottom = 750

grip_profile = [
    (grip_x, lower_bottom),
    (grip_x - 10, grip_bottom - 100),
    (grip_x - 15, grip_bottom - 50),
    (grip_x - 15, grip_bottom),
    (grip_x + 110, grip_bottom),
    (grip_x + 115, grip_bottom - 40),
    (grip_x + 115, lower_bottom + 50),
    (grip_x + 90, lower_bottom)
]

# Draw grip with gradient
for i in range(len(grip_profile) - 1):
    draw.line([grip_profile[i], grip_profile[i + 1]],
              fill=add_alpha(polymer_base, 255), width=3)
draw.polygon(grip_profile, fill=add_alpha(polymer_base, 255),
             outline=add_alpha(shadow, 255))

# Grip texture (stippling pattern)
for y in range(int(lower_bottom + 10), int(grip_bottom - 10), 15):
    for x in range(int(grip_x - 10), int(grip_x + 105), 12):
        if (y + x) % 30 < 15:
            draw.ellipse([x, y, x + 3, y + 3],
                        fill=add_alpha(polymer_highlight, 180))

# === BUFFER TUBE / STOCK ===
stock_x = 100
stock_y = barrel_y + 15
stock_diameter = 60

# Buffer tube (cylindrical)
for i in range(stock_diameter):
    shade = int(50 + 30 * math.sin(math.pi * i / stock_diameter))
    color = (shade, shade + 2, shade + 5, 255)
    draw.line([(stock_x, stock_y + i), (lower_start, stock_y + i)],
              fill=color, width=1)

# Stock buttpad
draw.rectangle([stock_x - 10, stock_y - 15, stock_x + 25, stock_y + stock_diameter + 15],
               fill=add_alpha((60, 50, 45), 255), outline=add_alpha(shadow, 255))

# Buttpad texture
for y in range(int(stock_y - 10), int(stock_y + stock_diameter + 10), 8):
    draw.line([(stock_x - 5, y), (stock_x + 20, y)],
              fill=add_alpha((40, 35, 30), 200), width=1)

# === CHARGING HANDLE ===
ch_x = lower_start + 200
draw.rectangle([ch_x, rail_y + 8, ch_x + 60, rail_y + 25],
               fill=add_alpha(worn_metal, 255), outline=add_alpha(shadow, 255))
# CH latch
draw.rectangle([ch_x + 50, rail_y + 10, ch_x + 58, rail_y + 23],
               fill=add_alpha(dark_metal, 255))

# === EJECTION PORT ===
port_x = receiver_start + 250
draw.rectangle([port_x, lower_top + 15, port_x + 120, lower_top + 55],
               fill=add_alpha((15, 15, 18), 255), outline=add_alpha(shadow, 255))

# === FINAL DETAILS ===
# Panel lines
draw.line([(lower_start + 50, lower_top + 10), (lower_end - 50, lower_top + 10)],
          fill=add_alpha(shadow, 180), width=2)
draw.line([(lower_start + 50, lower_bottom - 10), (lower_end - 50, lower_bottom - 10)],
          fill=add_alpha(shadow, 180), width=2)

# Forward assist
fa_x = receiver_start + 320
draw.ellipse([fa_x, lower_top + 15, fa_x + 30, lower_top + 45],
             fill=add_alpha(base_metal, 255), outline=add_alpha(shadow, 255))
for i in range(8):
    angle = i * math.pi / 4
    x1 = fa_x + 15 + 8 * math.cos(angle)
    y1 = lower_top + 30 + 8 * math.sin(angle)
    x2 = fa_x + 15 + 12 * math.cos(angle)
    y2 = lower_top + 30 + 12 * math.sin(angle)
    draw.line([(x1, y1), (x2, y2)], fill=add_alpha(dark_metal, 255), width=2)

# Apply smoothing for anti-aliasing
img = img.filter(ImageFilter.SMOOTH_MORE)

# Rotate the gun to point toward quadrant 2 (upper-left) with tilt for better FPS perspective
# The gun is currently horizontal (barrel points right/0°, grip points down/270°)
# We need: muzzle pointing to Q2 with 40° additional tilt (30° initial + 10° to the right)
# This requires a 265-degree counter-clockwise rotation (225° + 40° tilt)
angle = 265  # degrees counter-clockwise (225° base + 40° total tilt)
img_rotated = img.rotate(angle, expand=True, resample=Image.BICUBIC)

# Create a new canvas with some extra space
final_width = img_rotated.width
final_height = img_rotated.height
final_img = Image.new('RGBA', (final_width, final_height), (0, 0, 0, 0))

# Paste the rotated gun onto the canvas
final_img.paste(img_rotated, (0, 0), img_rotated)

# Save
final_img.save('assets/gun_sprite.png', optimize=True)
print("High-quality tactical rifle sprite created!")
print(f"Resolution: {final_width}x{final_height}")
print(f"Rotated 265 degrees counter-clockwise (225° base + 40° tilt, muzzle pointing Q2/upper-left with tilt to right)")
print("This sprite features:")
print("  - Realistic metal gradients and shading")
print("  - Detailed picatinny rail system")
print("  - Textured polymer grip")
print("  - Proper magazine with ribbing")
print("  - Front and rear sights")
print("  - Muzzle device with flash hider")
print("  - Natural FPS perspective angle")


