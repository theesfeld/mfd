# Distilled F-16 MFD format notes (public DCS F-16C EA Guide)

Source: Eagle Dynamics DCS F-16C Early Access Guide (public PDF).

Study only — not OEM classified TO art. HAF GR1F16CJ-1 defers MFD detail to GR1F16CJ-34-1-1.


## MULTI-FUNCTION DISPLAYS (MFD)
Two 4 × 4 inch color liquid crystal Multi-Function Displays (MFD) provide video and text presentations to the pilot
for the aircraft’s various sensors. The MFDs also serve as the primary interface to the aircraft’s external stores,
data transfer and loading equipment, and diagnostics for the aircraft systems and flight controls.
Each sensor or aircraft system can be accessed via their respective MFD “format”. Some MFD formats will include
multiple “pages” that can be selected to access additional options or settings. The options and settings associated
with the systems of each format or page are controlled through Option Select Buttons (OSBs) around the display
bezel of each MFD. Each OSB interacts with the text displayed next to it to toggle through functions or select a
different page. If the OSB text is highlighted, the option is enabled or the associated command is in progress.
Additionally, four rocker buttons are present on each MFD that allows the pilot to adjust the appearance of the
video and text on the MFD screen itself.
                                                    1. OSB 1→5



          2. GAIN Rocker                                                                    3. SYM Rocker




          1. OSB 16 ↑ 20                                                                     1. OSB 6 ↓ 10




                                                                                            5. CON Rocker
           4. BRT Rocker



                                                   1. OSB 15←11

1.   Option Select Button (OSB). Selects the option corresponding with the displayed text adjacent to the
     MFD button itself.
     •    OSB 1-5. The top row of Option Select Buttons are numbered from 1 starting on the far left to 5 on
          the far right.
     •    OSB 6-10. The right column of Option Select Buttons are numbered from 6 starting at the top to 10
          at the bottom.
     •    OSB 11-15. The bottom row of Option Select Buttons are numbered from 11 starting on the far right
          to 15 on the far left.
     •    OSB 16-20. The left column of Option Select Buttons are numbered from 16 starting at the bottom to
          20 at the top.


                                                                   EAGLE DYNAMI EAGLE DYNAMICS               121

 DCS                                [F-16C Viper]

2.   GAIN Rocker. Adjusts the intensity of the MFD sensor video. The video is adjusted independently of the
     symbology intensity or overall brightness/contrast settings of the MFD itself. If held continuously to either
     position, the video will continuously increment to the minimum or maximum allowable brightness settings.
     •    If the FCR MFD format is displayed and the FCR is set to Ground Map (GM) or Sea (SEA) mode, the
          GAIN rocker adjusts the intensity of the radar map underlay independently of the MFD symbology.
     •    If the FCR MFD format is displayed and the FCR is set to Ground Moving Target (GMT) mode, the GAIN
          rocker adjusts the gain of the Moving Target Indicators independently of the radar map underlay or
          the remaining MFD symbology
     •    If the TGP MFD format is displayed and the FLIR Gain Control Mode on the TGP Control page is set to
          Manual Gain Control (MGC) at OSB 18, the GAIN rocker adjusts the thermal gain setting of the targeting
          pod’s FLIR camera.
3.   SYM Rocker. Adjusts the intensity of the MFD symbology independently of the MFD sensor video or overall
     brightness/contrast settings of the MFD itself. If held continuously to either position, the symbology intensity
     will continuously increment to the minimum or maximum allowable settings.
4.   BRT Rocker. Adjusts the overall brightness setting of the MFD display. If held continuously to either
     position, the brightness setting will continuously increment to the minimum or maximum allowable settings.
5.   CON Rocker. Adjusts the overall contrast setting of the MFD display. If held continuously to either position,
     the contrast setting will continuously increment to the minimum or maximum allowable settings.




 122

                                                                      [F-16C Viper]          DCS


Format Selection Master Menu Page
The Format Selection Master Menu page is used to assign a specific MFD format to the Format Select buttons
(OSB 13, OSB 14, and OSB 15). Additionally, the RESET MENU page is accessed from Master Menu page, which
can be used to reset the MFD symbology, brightness and contrast settings to their default values.
                          1. BLANK Format    2. HAD Format    3. RCCE Format     4. RESET MENU Page

## Format Selection Master Menu Page
The Format Selection Master Menu page is used to assign a specific MFD format to the Format Select buttons
(OSB 13, OSB 14, and OSB 15). Additionally, the RESET MENU page is accessed from Master Menu page, which
can be used to reset the MFD symbology, brightness and contrast settings to their default values.
                          1. BLANK Format    2. HAD Format    3. RCCE Format     4. RESET MENU Page




         5. FCR Format                                                                                10. SMS Format



         6. TGP Format                                                                                11. HSD Format



         7. WPN Format                                                                                12. DTE Format



         8. TFR Format                                                                                13. TEST Format



         9. FLIR Format                                                                               14. FLCS Format




                               15. Swap Button    16. Format Select Buttons    17. Declutter Button


1.   BLANK Format. Assigns the BLANK MFD format to the highlighted Format Select button. When a Format
     Select button is assigned to the BLANK format, no text will be displayed above the OSB. The format selection
     corresponding with that OSB will be removed from the MFD format selection cycle when the DMS Left and
     DMS Right commands are used on the Side Stick Controller (SSC).
2.   HARM Attack Display (HAD) Format. Assigns the HAD MFD format to the highlighted Format Select
     button. The HAD format is used to operate the externally-mounted HARM Targeting System pod. The HTS
     pod is used for detection and geo-location of air defense radar systems. The HTS pod is most commonly
     used during the Suppression of Enemy Air Defenses (SEAD) mission and can hand-off specific threat radar
     emitters to AGM-88 HARM anti-radar missiles for engagement or generate target locations for other onboard
     sensors or weapons. (See ASQ-213 HARM Targeting System for more information.)
3.   RCCE Format. The Reconnaissance MFD format is not functional in the F-16C variant that is simulated by
     DCS: F-16C Viper.
4.   RESET MENU Format. Displays the Reset Menu page. This page includes options to reset the MFD to the
     default or pre-programmed values for symbology intensity, brightness and contrast. (N/I)
5.   Fire Control Radar (FCR) Format. Assigns the FCR MFD format to the highlighted Format Select button.
     The FCR format is used to operate the APG-68 radar system. The APG-68 is used in air-to-air mode for
     detection, tracking and engagement of hostile aircraft; and in air-to-ground mode for ground mapping,
     ranging, and detection and targeting of ground vehicles or maritime vessels. (See APG-68 Fire Control Radar
     for more information.)


                                                                          EAGLE DYNAMI EAGLE DYNAMICS                   123

 DCS                               [F-16C Viper]

6.   Targeting Pod (TGP) Format. Assigns the TGP MFD format to the highlighted Format Select button. The
     TGP format is used to operate externally mounted electro-optical sensor pods such as the AAQ-33. Targeting
     pods are used for medium to high altitude reconnaissance; optical detection and tracking of ground targets;
     or for designation of ground targets for engagement by precision guided munitions (PGM). (See AAQ-33
     Advanced Target Pod for more information.)
7.   Weapon (WPN) Format. Assigns the WPN MFD format to the highlighted Format Select button. The WPN
     format is used to relay sensor video and targeting data from munitions such as the AGM-65 TV/IR guided
     missiles or the AGM-88 HARM anti-radar missile so the pilot can directly control the respective missile’s
     targeting systems prior to weapons release. (See AGM-65 Maverick and AGM-88 HARM for more
     information.)
8.   TFR Format. The Terrain Following Radar MFD format is not functional in the F-16C variant that is simulated
     by DCS: F-16C Viper.
9.   FLIR Format. The Forward Looking Infrared MFD format is not functional in the F-16C variant that is
     simulated by DCS: F-16C Viper.
10. Stores Management System (SMS) Format. Assigns the SMS MFD format to the highlighted Format
    Select button. The SMS format is used to select different munitions for employment, select and modify
    weapon release profiles, set warhead fuzing, and adjust terminal attack parameters. (See the Tactical
    Employment chapter for more information.)
11. Horizontal Situation Display (HSD) Format. Assigns the HSD MFD format to the highlighted Format
    Select button. The HSD format provides the pilot with a top-down view of the battlespace around the aircraft
    to include navigational data, airspace and tactical boundaries, air defense threats, and fuses onboard radar
    data with tactical information derived from allied aircraft (such as other flight members and AWACS). (See
    the Tactical Employment chapter for more information.)
12. Data Transfer Equipment (DTE) Format. Assigns the DTE MFD format to the highlighted Format Select
    button. The DTE format is used to upload pre-planned mission data and aircraft configuration settings from
    the cockpit Data Transfer Unit (DTU) into the MMC memory.
13. Test (TEST) Format. Assigns the TEST MFD format to the highlighted Format Select button. The TEST
    format is used to display the Maintenance Fault List (MFL) and perform Built-In Tests (BIT) during system
    diagnostics and maintenance procedures. (N/I)
14. Flight Control System (FLCS) Format. Assigns the FLCS MFD format to the highlighted Format Select
    button. The FLCS format is used to display data from of Flight Control Computer (FLCC). (N/I)
15. Swap Button. Pressing this button will swap the currently displayed MFD formats between the left and
    right MFDs. In addition, the MFD formats assigned to each Format Select Button will be swapped as well.
16. Format Select Buttons. Selects the corresponding MFD format for display on the MFD. When the Format
    Selection Master Menu page is displayed, selecting the OSB will highlight the text above it and enable a new
    format to be assigned to that button. If the text displayed above the OSB is already highlighted, pressing
    the same OSB will leave the Format Selection Master Menu page and display the MFD format that is assigned
    to that button.
17. Declutter Button. Removes the text symbology adjacent to each corresponding OSB on the MFD.
    However, the associated commands for each OSB will still remain. (N/I)




 124

                                                               [F-16C Viper]   DCS


Re-assigning MFD Formats
Each of the seven avionics master modes (Navigation, Air-to-Air, Air-to-Ground, Missile Override, Dogfight,
Selective Jettison, and Emergency Jettison) are initialized with pre-configured MFD formats assigned to each
Format Select button of each MFD. These MFD format assignments can be re-configured by the pilot at any time
via the Format Selection Master Menu page.
To assign a different format to a Format Select button (OSB 12,
OSB 13 or OSB 14) on either MFD, set the avionics to the master
mode that is meant to be edited, and perform the following:
1.   If the MFD text above the Format Select OSB that is intended
     to be re-assigned to a different MFD format is already
     highlighted, press that same OSB to open the Format
     Selection Master Menu page.
     If the MFD text above the Format Select OSB that is intended

## Horizontal Situation Display (HSD) MFD Format
The HSD MFD format presents tactical symbols representing the positions of flight members, hostile aircraft, air
defenses, and sensor information overlaid onto navigation information such as steerpoints and routes. Many of
these symbology elements can be selectively toggled on the HSD Control page and are meant to enhance and
maintain the pilot’s situational awareness of the tactical environment.
                                2. FCR Range Coupling   3. Field-Of-View   4. Message Page


            1. Centered/                                                                     5. Control Page
          Depressed Format


             6. Range Rings                                                                  13. Navigation Steerpoint


                                                                                             14. Navigation Route

             7. Range Scale

                                                                                             15. Freeze

       8. FCR Search Volume
                                                                                             16. Geographic Line

         9. Ghost A-A Cursor


       10. Ghost A-A Cursor                                                                  17. Pre-planned Threat
         Bearing & Range

                                                                                             18. Cursor Zero
               11. Ownship


              12. Aircraft                                                                   19. Destination Steerpoint
           Reference Symbol




1.   Centered/Depressed Format. Toggles between Depressed (DEP) and Centered (CEN) HSD formats.
     When set to Depressed, the ownship is biased to the bottom portion of the HSD, allowing the HSD to
     primarily depict battlespace in front of the aircraft. When set to Centered, the ownship is displayed in the
     center of the HSD, depicting battlespace in all directions around the aircraft equally.
2.   FCR Range Coupling. Toggles between Decoupled (DCPL) and Coupled (CPL) HSD modes. When set to
     Decoupled (DCPL) mode, the FCR range scale will have no effect on the HSD range scale, allowing the range
     scales of each MFD format to be adjusted independently of the other.
     When set to CPL, the HSD range scale will be correlated to match the FCR range scale when in Centered
     format or to 1.5x the range of the FCR range scale when in Depressed format (one additional magenta
     range ring in front of the FCR search volume). CPL mode is overridden any time the HSD is SOI, allowing
     the pilot to “bump” the HSD range scale independently of the FCR range scale. Once the HSD is no longer
     SOI, the HSD will revert to CPL mode.


 326

                                                                [F-16C Viper]       DCS


3.   Field-Of-View. Cycles the HSD between NORM, EXP1, and EXP2 fields-of-view when the HSD is SOI. The
     Expand/FOV button on the Side Stick Controller (SSC) may also be pressed to cycle between the HSD fields-
     of-view when the HSD is SOI. (See Expand Field-Of-View for more information.)
4.   Message Page (MSG). Toggles the MFD between the HSD base page and the HSD Message page. (N/I)
5.   Control Page (CNTL). Toggles the MFD between the HSD base page and the HSD Control page.
6.   Range Rings. Depicts range from the ownship and the cardinal directions of north, east, south, and west
     (referenced from magnetic north) from the innermost range ring.
     When the HSD is set to Depressed (DEP) format, the outer ring will correspond with the HSD range scale,
     with two additional inner rings at ⅔ and ⅓ of the range scale. When the HSD is set to Centered (CEN)
     format, the outer ring will correspond with the HSD range scale, with an inner ring at ½ the range scale.
     Magnetic north is depicted as an arrow protruding outward from the innermost ring, south is depicted as a
     long tick mark straddling the innermost ring, and east and west are depicted as short tick marks protruding
     inward from the innermost ring.
7.   Range Scale. Adjusts the scale of the HSD up or down, with the current range scale setting (in nautical
     miles) displayed between the arrow buttons. The HSD range scale corresponds with the outermost range
     ring depicted on the HSD and is scaled based on the DEP/CEN format selection. The available HSD scales
     are shown below:
     Depressed (DEP)                 15 NM          30 NM          60 NM          120 NM         240 NM
     Centered (CEN)                  10 NM          20 NM          40 NM          80 NM          160 NM
     When the HSD is set to its highest or lowest range scales, the upper or lower range scale arrows are
     removed, respectively.
8.   FCR Search Volume. Depicts the lateral boundaries of the fire control radar scan pattern in azimuth and
     range, based on the current azimuth scan width, range scale, and position of the FCR Acquisition Cursor.
9.   Ghost A-A Cursor. When the opposite MFD displays the FCR format and the FCR is set to Combined Radar
     Mode (CRM), the location of the FCR Acquisition Cursor relative to the ownship will be displayed on the HSD.
     This allows the pilot to correlate FCR target positions with the overall tactical situation depicted on the HSD.
10. Ghost A-A Cursor Bearing & Range. When the Ghost A-A cursor is displayed, this data field will display
    the bearing (in degrees magnetic) and range (in nautical miles) from the currently selected steerpoint to
    the Ghost A-A cursor. If Bullseye is enabled on the BULL DED page, this data field will display the bearing
    and range from the Bullseye steerpoint to the Ghost A-A cursor.
11. Ownship. Depicts the location of the ownship.
12. Aircraft Reference Symbol. Displays the relative alignment of the aircraft heading with the selected
    steerpoint, System Point-of-Interest (SPI), or weapon release solution. If the line is to the left or right of the
    watermark, the pilot must turn left or right respectively toward the vertical line to align the aircraft on course
    toward the selected steerpoint, SPI, or weapon release solution.
13. Navigation Steerpoint. Steerpoints 1-25 composing a navigation route are displayed as circles for normal
    steerpoints, squares for initial points, and triangles for targets. The steerpoint selected for navigation is
    displayed as a solid symbol; all other steerpoints are displayed as hollow symbols. Navigation steerpoints
    are displayed as white within the active navigation route and gray within the non-active navigation routes,
    if present.
     Navigation steerpoints that are not part of a navigation route are not displayed on the HSD unless they are
     the selected steerpoint.
14. Navigation Route. Navigation routes are displayed as solid lines linking sequential steerpoints 1-25. The
    active navigation route is displayed as white and the non-active navigation routes, if present, are displayed
    as gray. (See Navigation Routes for more information.)


                                                                                     EAGLE DYNAMICS        327

 DCS                               [F-16C Viper]


15. Freeze (FZ). Freezes the HSD independently of the Ownship position, indicated by the highlighted “FZ”
    text adjacent to OSB 7. If the HSD is SOI when OSB 7 is pressed, the HSD will enter Centered (CEN) format
    on the location of the HSD cursor. If the HSD is not SOI when OSB 7 is pressed, the HSD will enter Centered
    (CEN) format on the location of the Ownship.
     A second press of OSB 7 will unfreeze the HSD. If the HSD was set to Depressed (DEP) format prior to being
     frozen, the HSD will revert to DEP format.
     NOTE: The Freeze (FZ) option is inhibited if the HSD is set to Expand (EXP1/EXP2) fields-of-view.
16. Geographic Line. Geographic lines are depicted as dashed lines linking sequential steerpoints 31-55. These
    lines may be used to depict airspace boundaries, kill boxes, the Forward Line of Own Troops (FLOT) or a

## SMS Inventory (INV) Page
An Inventory page is available that shows external stores loaded on each station. When the aircraft master mode
is set to Navigation, Selective Jettison, or Emergency Jettison modes, the SMS Inventory page is displayed as the
base page. When the aircraft master mode is set to Air-to-Air Missile, Air-to-Ground, Missile Override, or Dogfight
modes, the INV page may be accessed from each respective base page by pressing the INV button (OSB 4).




      1. SMS Operating Mode


2. Gun Ammunition Quantity


     3. Gun Ammunition Type                                                                4. External Stores Wingform




                                                                                           5. Selective Jettison Page


                                  SMS Inventory Page – Navigation mode


1.     SMS Operating Mode. Displays the operating mode of the of the Stores Management System.
2.     Gun Ammunition Quantity. Displays the remaining ammunition quantity onboard for the M61 20mm
       rotary cannon, in 10 round increments (e.g. “51” indicates 510 rounds remaining).
3.     Gun Ammunition Type. Displays the type of ammunition loaded into the internal ammunition drum. “M56”
       will be displayed for any M50-series ammunition. “PGU-28” will be displayed for any PGU-series ammunition.
4.     External Stores Wingform. Displays external stores installed on underwing and center fuselage pylons,
       including with any associated missile launchers or bomb racks.
5.     Selective Jettison Page. Selects Selective Jettison mode, overriding the current aircraft master mode.


 334

                                                               [F-16C Viper]      DCS


In addition to the gun ammunition type displayed in the top left corner of the inventory page, the SMS will use a
series of weapon and equipment codes to indicate specifically what external stores are loaded onto the underwing
and centerline fuselage stations on the aircraft. A list of these codes is provided on the following page.
Stations 1, 2, 8, and 9 may only be equipped with LAU-129 air-to-air missile rails. These stations are displayed in
a two-line format, with the LAU-129 Missile Rail Launcher on the first line and the corresponding air-to-air weapon
on the second line.




                                        SMS Inventory Page Layout

Stations 3, 4, 5, 6 and 7 may be equipped with a variety of external stores, including air-to-air or air-to-ground
munitions, fuel tanks, and ECM or travel pods. These stations are displayed in a three-line format. Depending on
the combination of external munitions or equipment that is installed on these stations, the station data may be
composed of one, two or three lines of data.
In the example above, stations 3, 5, and 7 are installed with a MAU-12 Ejector Rack. However, the MAU-12
installed on station 3 is carrying a TER-9/A Triple Ejector Rack loaded with a pair of GBU-12 laser-guided bombs
and station 7 is carrying a BRU-57/A Smart Multiple Carriage Rack loaded with a pair of GBU-38 inertially-aided
bombs, whilst the MAU-12 on station 5 is simply carrying an ECM pod.




                                                                                   EAGLE DYNAMICS        335

 DCS                            [F-16C Viper]


SMS Weapon/External Stores Codes
 CODE    MUNITION/EQUIPMENT                       CODE    MUNITION/EQUIPMENT
 M56     M50-series 20mm ammunition               MAU     MAU-12 Ejector Rack
 PGU28   PGU-series 20mm ammunition               TER     TER-9/A Triple Ejector Rack
                                                  MRL     LAU-129A/A Missile Rail Launcher
 TA9LM   CAP-9M Captive Air Training Missile      L03     LAU-3/A 19-tube Rocket Launcher
 A-9J    AIM-9P IR-guided missile                 L68     LAU-68D/A 7-tube Rocket Launcher
 A-9NP   AIM-9P3 IR-guided missile                L131    LAU-131/A 7-tube Rocket Launcher
 A-9LM   AIM-9P5, -9L, or -9M IR-guided missile   L88A    LAU-88/A Triple Rail Missile Launcher
 A-9X    AIM-9X IR-guided missile                 L117    LAU-117A(V)3/A Maverick Missile Launcher
 A120B   AIM-120B active radar-guided missile     L118    LAU-118(V)2/A Guided Missile Launcher
 A120C   AIM-120C active radar-guided missile     BRU     BRU-57/A Smart Multiple Carriage Rack


 ACMI    AN/ASQ-T50 TCTS pod                      TK300   300-gallon external centerline tank
 AL131   AN/ALQ-131 ECM pod                       TK370   370-gallon external wing tank
 AL119   AN/ALQ-184 ECM pod
                                                  GB12    GBU-12 or BDU-50LGB laser-guided bomb
 BD33T   BDU-33 practice bomb                     GB10C   GBU-10C/B laser-guided bomb
 B49     Mk-82 AIR or BDU-50HD with BSU-49        GB24A   GBU-24A/B laser-guided bomb
 M82     Mk-82 or BDU-50LD bomb                   GB31A   GBU-31(V)1/B INS/GPS-guided bomb
 M82S    Mk-82 bomb with Mk15 Snakeye pedals      GB31B   GBU-31(V)3/B INS/GPS-guided bomb
 M84     Mk-84 bomb                               GB38    GBU-38 INS/GPS-guided bomb
 B50     Mk-84 AIR bomb with BSU-50
 BD50    Mk-84 AIR practice bomb with BSU-50      CB103   CBU-103 INS/GPS-guided cluster bomb
                                                  CB105   CBU-105 INS/GPS-guided cluster bomb
 CB87B   CBU-87 with 202 BLU-97B submunitions
 CB97B   CBU-97 with 40 BLU-108 submunitions      AG65D   AGM-65D IR-guided missile 125lb warhead
                                                  AG65G   AGM-65G IR-guided missile 300lb warhead
 M151    M151 high explosive rockets              AG65H   AGM-65H TV-guided missile 125lb warhead

## Fire Control Radar (FCR) MFD Format
The FCR MFD format is the primary interface with the fire control radar and presents radar targeting data and
ground map imagery to the pilot in either an air-to-air format or an air-to-ground format. When the FCR air-to-
air format is displayed, airborne targets detected by the ownship FCR are displayed along with airborne targets
received via datalink from offboard sources. When the FCR air-to-ground format is displayed, radar-generated
imagery of the surface and/or any moving ground targets are displayed.
See the Tactical Net Datalink chapter for more information regarding datalink target symbols displayed on the
FCR MFD format.
FCR Air-to-Air Format
When displayed in air-to-air mode, the FCR displays radar returns of aircraft in a B-Scope format, in which the
position of the ownship is centered along the bottom edge of the FCR display area. Radar targets are displayed
laterally within the FCR display area based on azimuth in relation to the ownship nose, and vertically within the
FCR display based on range or closure velocity.
                            1. FCR Mode   2. FCR Sub-mode   3. Field-Of-View   4. Standby Override   5. Control Page




                                                                                                              6. SMDL Mode

                   7. Range Scale




           8. Azimuth Scan Width                                                                              10. Horizon Line

                                                                                                                  11. A-A
             9. Elevation Bar Scan                                                                            Acquisition Cursor


                  12. FCR Cursor
                  Bearing & Range
                                                                                                              16. Weapon Status
             13. AIFF Mode Status


     14. Aircraft Reference Symbol


      15. Datalink Declutter Level

1.     FCR Mode (Air-to-Air format). Displays the FCR Mode Menu page. The current FCR mode is displayed
       below OSB 1.
2.     FCR Sub-mode. Selects the air-to-air sub-mode when the FCR is not in a tracking sub-mode. The current
       air-to-air sub-mode is displayed below OSB 2.
       If the FCR mode is set to CRM when OSB 2 is pressed, the OSB selection will advance to the next sub-mode
       in a cyclic manner: RWS → VSR → TWS → RWS.
       •      RWS. The FCR is set to Range While Scan sub-mode.
       •      VSR. The FCR is set to Velocity Search with Ranging sub-mode.
       •      TWS. The FCR is set to Track While Scan sub-mode.



 362

                                                              [F-16C Viper]      DCS


     If the FCR mode is set to ACM when OSB 2 is pressed, the OSB selection will advance to the next sub-mode
     in a cyclic manner: 20 → 60 → SLEW → BORE → 20.
     •    20. The FCR is set to 30×20 sub-mode.
     •    60. The FCR is set to 10×60 sub-mode.
     •    SLEW. The FCR is set to SLEW sub-mode.
     •    BORE. The FCR is set to BORE sub-mode.
3.   Field-Of-View. Cycles the field-of-view of the FCR MFD format when the FCR is the Sensor-Of-Interest
     (SOI). The current field-of-view is displayed below OSB 3.
     If the FCR is set to CRM, GMT, or SEA when OSB 3 is pressed, the OSB selection will toggle between NORM
     and EXP. (See Expand Field-of-View in the FCR Air-to-Air Modes section for more information.)
     If the FCR is set to GM when OSB 3 is pressed, the OSB selection will advance to the next field-of-view in a
     cyclic manner: NORM → EXP → DBS1 → DBS2 → NORM. (See Expand Field-of-View in the FCR Air-to-
     Ground Modes section for more information.)
     •    NORM. The FCR MFD format is set to the normal, unexpanded display area.
     •    EXP. The FCR MFD format is expanded at a 4:1 display ratio.
     •    DBS1. The FCR MFD format is expanded at a 4:1 display ratio. Doppler Beam Sharpening radar
          processing is enabled to increase radar image resolution. DBS1 is only available in GM mode.
     •    DBS2. The FCR MFD format is expanded at a ratio dependent on the range to the radar cursor. Doppler
          Beam Sharpening radar processing is enabled to increase radar image resolution. DBS2 is only available
          in GM mode.
     The Expand/FOV button on the Side Stick Controller (SSC) may also be pressed to cycle the FCR field-of-
     view when the FCR is SOI.
4.   Standby Override (OVRD). Sets the FCR to Standby mode. When Standby Override is enabled, the text
     below OSB 4 will be highlighted in white. When Standby Override is disabled, the FCR returns to the last
     FCR mode that it was set to within the current master mode prior to Standby Override being enabled.
5.   Control Page (CNTL). Toggles the MFD between the FCR base page and the FCR Control page.
6.   SMDL Mode. Not implemented.
7.   Range Scale. Adjusts the scale of the FCR up or down, with the current range scale setting (in nautical
     miles) displayed between the arrow buttons. The maximum and minimum ranges that may be selected for
     each FCR mode are shown below:
     FCR Mode              CRM                  ACM (Bore)           ACM (20/60/Slew)         GM/GMT/SEA
     Maximum Range         160 NM               40 NM                10 NM                    80 NM
     Minimum Range         5 NM                 5 NM                 10 NM                    10 NM
     When the FCR is set to its highest or lowest range scales, the upper or lower range scale arrows are removed,
     respectively. If the Acquisition Cursor is slewed beyond the upper or lower limits of the current range scale
     using the RDR CURSOR/ENABLE switch when the FCR is not in a tracking sub-mode, the range scale will be
     “bumped” to the next higher or next lower range scale setting in sequence.
8.   Azimuth Scan Width. Selects the horizontal radar scan size in azimuth when the FCR is in Combined Radar
     Mode (CRM) without a designated Bugged Target (FCR TOI), or Ground Map (GM) or Sea (SEA) modes.
     Separate Azimuth Scan Width settings are retained for each CRM sub-mode and are automatically set when
     returning to the corresponding sub-mode. The current setting is displayed to the right of OSB 18. Each press
     of OSB 18 will advance to the next azimuth scan width setting in a cyclic manner: A6 → A3 → A1 → A6.



                                                                                  EAGLE DYNAMICS        363

 DCS                                 [F-16C Viper]


     •    A6. The FCR is scanning ±60° to either side of the aircraft nose.
     •    A3. The FCR is scanning ±30° to either side of the Acquisition Cursor.
     •    A2. The FCR is scanning ±25° to either side of the Acquisition Cursor. This setting is only available
          when the FCR is set to TWS sub-mode with a designated Bugged Target or Cursor Target.
     •    A1. The FCR is scanning ±10° to either side of the Acquisition Cursor.
     If the Acquisition Cursor is slewed to the left or right boundary of the MFD display area using the RDR
     CURSOR/ENABLE switch when the FCR is not in a tracking sub-mode, the azimuth scan width will be
     “bumped” between A6 and A3 settings.
9.   Elevation Bar Scan. Selects the vertical radar scan size in elevation when the FCR is in Combined Radar
     Mode (CRM) without a designated Bugged Target (FCR TOI). Separate Elevation Bar Scan settings are
     retained for each CRM sub-mode and are automatically set when returning to the corresponding sub-mode.
     The current setting is displayed to the right of OSB 17. Each press of OSB 17 will advance to the next
     elevation bar scan setting in a cyclic manner: 4B → 2B → 1B → 4B.
     •    4B. The FCR is scanning 4 bars in elevation.
     •    3B. The FCR is scanning 3 bars in elevation. This setting is only available when the FCR is set to TWS
          sub-mode with a designated Bugged Target or Cursor Target.
     •    2B. The FCR is scanning 2 bars in elevation.
     •    1B. The FCR is scanning 1 bar in elevation.
10. Horizon Line. Indicates the aircraft attitude in pitch and roll to aid the pilot in maintaining spatial orientation
    when focused inside the cockpit. If the aircraft nose is on the horizon in level flight, the Horizon Line will be
    centered and parallel with the upper and lower edges of the MFD display area, with the two vertical tick
    marks on the outer edges of the Horizon Line indicating the direction toward the ground. If the nose is
    above the horizon, the Horizon Line will be displaced toward the bottom of the MFD but will remain at the
    bottom of the display at +60° pitch or beyond. If the nose is below the horizon, the Horizon Line will be
    displaced toward the top of the MFD but will remain at the top of the display at -60° pitch or beyond. If the
    aircraft banks left or right, the Horizon Line will rotate in opposite direction to remain level with the horizon.
11. A-A Acquisition Cursor. The A-A Acquisition Cursor is slewed using the RDR CURSOR/ENABLE switch and
    is used to designate target symbols or steer the FCR search volume left or right, or control the azimuth and
    range settings the FCR. Two numerical values are displayed to the right of the cursor, corresponding with
    the upper and lower altitude limits (in thousands of feet above mean sea level, or MSL) of the FCR search
    volume at the position of the cursor itself, based on the ownship altitude, antenna elevation setting, and
    elevation bar scan setting. The upper limit is displayed in blue to indicate a positive altitude and red to
    indicate a negative altitude. The lower limit is displayed in white to indicate a positive altitude and red to
    indicate a negative altitude.
12. FCR Cursor Bearing & Range. Displays the bearing (in degrees Magnetic) and range (in nautical miles)
    from the selected steerpoint to the FCR cursor or Bugged Target (FCR TOI). If Bullseye is enabled on the
    BULL DED page, this data field will display the bearing and range from the Bullseye steerpoint to the FCR
    cursor or the Bugged Target (FCR TOI). (See “Bullseye” Reference Point for more information.)
13. AIFF Mode Status. Displays the IFF modes that are selected for interrogation by the AIFF antenna array.
    (See Advanced Identification-Friend-or-Foe for more information.)
14. Aircraft Reference Symbol. Displays the relative alignment of the aircraft heading with the selected
