// CMFD universal enclosure — F-16 spirit + late-80s industrial skin
// Issue #137 · theesfeld/mfd
// Units: mm
// Export: openscad -o ../print/PART.stl -D 'PART="front"' cmfd_enclosure.scad

/* [Export] */
PART = "front"; // ["front", "rear", "tray", "osb_cap", "rocker", "bumper", "assembly"]

/* [Envelope — spirit match, not exact LRU] */
outer_xy = 148;       // outer bezel width/height (≈5.83")
outer_z = 58;         // depth for SoM + battery + ports
wall = 3.2;           // thick wall for bag-toss durability
corner_r = 8;         // Walkman/Gundam soft radius
glass_xy = 102;       // ~4.0" active aperture
glass_frame = 3;      // lip under cover glass
bezel_front_z = 14;   // front face thickness
rear_z = outer_z - bezel_front_z;

/* [OSB layout — MUST match hardware/tools/kicad_build.py Board A] */
osb_count_side = 5;
osb_span = 78;        // KiCad osb_layout span (mm)
osb_hole = 9.2;       // switch cap shaft hole
osb_cap_xy = 12;
osb_cap_z = 6;
osb_inset = 14;       // KiCad inset from outer edge

/* [Rocker — KiCad rk positions at 26 mm from edges] */
rocker_w = 14;
rocker_h = 10;
rocker_inset = 26;

/* [Ports — rear deck] */
port_z = 12;          // height of port window band from bottom

$fn = 48;

module rounded_cube(size, r) {
    x = size[0]; y = size[1]; z = size[2];
    hull() {
        for (ix = [r, x - r])
            for (iy = [r, y - r])
                translate([ix, iy, 0])
                    cylinder(h = z, r = r);
    }
}

module shell_outer(z) {
    rounded_cube([outer_xy, outer_xy, z], corner_r);
}

module shell_inner(z, inset = wall) {
    translate([inset, inset, -0.1])
        rounded_cube([outer_xy - 2 * inset, outer_xy - 2 * inset, z + 0.2], max(1, corner_r - inset));
}

// ---- OSB centers: matching software clockwise 1..20 from top-left ----
function osb_centers() =
    let (
        c = outer_xy / 2,
        s = osb_span / 2,
        step = osb_span / 4,
        top = [for (i = [0:4]) [c - s + i * step, outer_xy - osb_inset]],
        // right 6..10 top→bottom
        right = [for (i = [0:4]) [outer_xy - osb_inset, c + s - i * step]],
        // bottom physical L→R = OSB 15,14,13,12,11
        bot = [for (i = [0:4]) [c - s + i * step, osb_inset]],
        // left top→bottom = 20,19,18,17,16
        left = [for (i = [0:4]) [osb_inset, c + s - i * step]]
    )
    concat(top, right, bot, left);

module osb_holes() {
    for (p = osb_centers())
        translate([p[0], p[1], -0.1])
            cylinder(h = bezel_front_z + 1, d = osb_hole);
}

module rocker_slots() {
    positions = [
        [rocker_inset, outer_xy - rocker_inset],                 // GAIN UL
        [outer_xy - rocker_inset, outer_xy - rocker_inset],     // SYM UR
        [rocker_inset, rocker_inset],                           // BRT LL
        [outer_xy - rocker_inset, rocker_inset]                 // CON LR
    ];
    for (p = positions)
        translate([p[0], p[1], bezel_front_z / 2])
            cube([rocker_w, rocker_h, bezel_front_z + 2], center = true);
}

module glass_aperture() {
    c = (outer_xy - glass_xy) / 2;
    translate([c, c, -0.1])
        cube([glass_xy, glass_xy, bezel_front_z + 1]);
}

module glass_recess() {
    // lip for cover glass from front
    c = (outer_xy - glass_xy) / 2 - 1.5;
    translate([c, c, bezel_front_z - 2])
        cube([glass_xy + 3, glass_xy + 3, 2.2]);
}

// ========== FRONT BEZEL ==========
module front_bezel() {
    difference() {
        union() {
            shell_outer(bezel_front_z);
            // slight raised rim — Discman/Walkman cue
            translate([2, 2, bezel_front_z - 0.6])
                rounded_cube([outer_xy - 4, outer_xy - 4, 0.6], corner_r - 2);
        }
        glass_aperture();
        glass_recess();
        osb_holes();
        rocker_slots();
        // screw bosses clearances to rear (4x M3)
        for (p = [[8, 8], [outer_xy - 8, 8], [outer_xy - 8, outer_xy - 8], [8, outer_xy - 8]])
            translate([p[0], p[1], -0.1])
                cylinder(h = bezel_front_z + 1, d = 3.4);
        // panel line accents (Gundam vibe) — shallow grooves
        for (a = [25, 50, 75, 100, 125])
            translate([a, wall / 2, bezel_front_z - 0.4])
                cube([0.6, outer_xy - wall, 0.5]);
    }
    // screw bosses
    for (p = [[8, 8], [outer_xy - 8, 8], [outer_xy - 8, outer_xy - 8], [8, outer_xy - 8]])
        difference() {
            translate([p[0], p[1], 0])
                cylinder(h = bezel_front_z, d = 8);
            translate([p[0], p[1], -0.1])
                cylinder(h = bezel_front_z + 1, d = 3.4);
        }
}

// ========== REAR SHELL ==========
module rear_shell() {
    difference() {
        shell_outer(rear_z);
        // main cavity
        translate([0, 0, wall])
            shell_inner(rear_z - wall + 0.2, wall);
        // front mating lip
        translate([wall / 2, wall / 2, rear_z - 3])
            rounded_cube([outer_xy - wall, outer_xy - wall, 3.2], corner_r - 1);
        // port windows — bottom rear face (y=0 side extruded)
        // USB-C x2
        translate([30, -0.1, port_z])
            cube([10, wall + 1, 4]);
        translate([48, -0.1, port_z])
            cube([10, wall + 1, 4]);
        // RJ45
        translate([70, -0.1, port_z - 1])
            cube([16, wall + 1, 10]);
        // multipin CAN/UART
        translate([95, -0.1, port_z])
            cube([14, wall + 1, 6]);
        // audio
        translate([115, -0.1, port_z + 1])
            cylinder(h = wall + 1, d = 6.5, $fn = 24);
        // battery door opening (back face z)
        translate([outer_xy / 2 - 22, outer_xy / 2 - 40, -0.1])
            cube([44, 80, wall + 0.2]);
        // screw holes
        for (p = [[8, 8], [outer_xy - 8, 8], [outer_xy - 8, outer_xy - 8], [8, outer_xy - 8]])
            translate([p[0], p[1], -0.1])
                cylinder(h = rear_z + 1, d = 3.4);
        // ventilation slots (side)
        for (i = [0:4])
            translate([-0.1, 40 + i * 12, 25])
                cube([wall + 0.2, 8, 3]);
    }
    // board standoffs Board B
    for (p = [[24, 40], [100, 40], [24, 100], [100, 100]])
        if (p[0] < outer_xy - 10 && p[1] < outer_xy - 10)
            difference() {
                translate([p[0], p[1], wall])
                    cylinder(h = 8, d = 7);
                translate([p[0], p[1], wall - 0.1])
                    cylinder(h = 9, d = 2.6);
            }
    // 1/4-20 insert pocket rear
    difference() {
        translate([outer_xy / 2, outer_xy - 18, wall])
            cylinder(h = 10, d = 14);
        translate([outer_xy / 2, outer_xy - 18, wall + 2])
            cylinder(h = 9, d = 8.5);
    }
}

// ========== BATTERY TRAY ==========
module battery_tray() {
    // holds 2x 18650 side by side
    tw = 42;
    th = 78;
    tz = 22;
    difference() {
        rounded_cube([tw, th, tz], 3);
        // cell pockets
        for (i = [0, 1])
            translate([6 + i * 18, 6, 3])
                cube([15, 66, 20]);
        // spring wire channels
        translate([4, 2, 10])
            cube([tw - 8, 3, 3]);
        translate([4, th - 5, 10])
            cube([tw - 8, 3, 3]);
        // pull tab slot
        translate([tw / 2 - 6, th - 1, 8])
            cube([12, 2, 10]);
    }
}

// ========== OSB CAP ==========
module osb_cap() {
    // hard plastic keycap — tactile feel priority
    difference() {
        union() {
            rounded_cube([osb_cap_xy, osb_cap_xy, 2.5], 1.2);
            translate([osb_cap_xy / 2, osb_cap_xy / 2, 0])
                cylinder(h = osb_cap_z, d = osb_hole - 0.3);
        }
        // switch stem pocket (6x6 switch)
        translate([osb_cap_xy / 2, osb_cap_xy / 2, osb_cap_z - 3.5])
            cylinder(h = 4, d = 3.6);
        // legend recess
        translate([1.5, 1.5, 2.0])
            rounded_cube([osb_cap_xy - 3, osb_cap_xy - 3, 0.6], 0.8);
    }
}

// ========== ROCKER ==========
module rocker() {
    difference() {
        hull() {
            translate([0, 0, 2])
                rounded_cube([rocker_w + 2, rocker_h + 2, 2], 1);
            translate([1, 1, 0])
                rounded_cube([rocker_w, rocker_h, 5], 1);
        }
        // pivot channel
        translate([(rocker_w + 2) / 2, -0.1, 2])
            rotate([-90, 0, 0])
                cylinder(h = rocker_h + 3, d = 1.8);
        // actuator nubs underside
        translate([4, (rocker_h + 2) / 2, -0.1])
            cylinder(h = 2, d = 2);
        translate([rocker_w - 2, (rocker_h + 2) / 2, -0.1])
            cylinder(h = 2, d = 2);
    }
}

// ========== CORNER BUMPER (TPU) ==========
module corner_bumper() {
    difference() {
        translate([0, 0, 0])
            rounded_cube([22, 22, outer_z - 4], 4);
        translate([6, 6, -0.1])
            cube([20, 20, outer_z]);
        // clip slot
        translate([3, 3, outer_z / 2 - 4])
            cube([4, 10, 8]);
    }
}

// ========== assembly preview ==========
module assembly() {
    color([0.75, 0.82, 0.9]) front_bezel();
    color([0.55, 0.62, 0.72]) translate([0, 0, -rear_z + 1]) rear_shell();
    color([0.2, 0.2, 0.25])
        translate([(outer_xy - glass_xy) / 2, (outer_xy - glass_xy) / 2, 2])
            cube([glass_xy, glass_xy, 1.5]);
    color([0.3, 0.45, 0.75])
        translate([outer_xy / 2 - 21, outer_xy / 2 - 39, -rear_z + wall + 2])
            battery_tray();
}

if (PART == "front") front_bezel();
else if (PART == "rear") rear_shell();
else if (PART == "tray") battery_tray();
else if (PART == "osb_cap") osb_cap();
else if (PART == "rocker") rocker();
else if (PART == "bumper") corner_bumper();
else if (PART == "assembly") assembly();
else front_bezel();
