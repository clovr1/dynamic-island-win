Bloop 0.22.0
============
Close the running island first (tray icon -> Settings -> Quit), then:
  Settings -> Updates -> "Install from a file" -> Choose file... -> Bloop.exe
(If Windows blocks the file: right-click -> Properties -> Unblock.)

SOUNDS (original synthesis, in the spirit of the iPhone's system sounds)
  notification   "Milestone": three notes, F - C - F, rising quickly and ringing (an imitation made from the recording you picked)
  small bubble   "Droplet": a very short soft "plip" (while music plays, a notification is a small bubble)
  status pills   each one has its own sound, in families:
                   Wi-Fi, Bluetooth, hotspot, cellular: two bright notes - rising = on, falling = off (each with its own notes)
                   airplane mode: three soft notes, falling = on, rising = off
                   Do Not Disturb: a low calm pair (down = on, up = off)
                   night light: one warm note gliding up (on) / down (off)      energy saver: one mellow note
                   battery: charging "bloop - ding", unplugged (the same turned around), low battery (two low gentle taps)
  pause "tok", play a light tick, skip two quick ticks, hovering silent
Settings -> Look: "Sound effects" (switch) and "Volume". All the files are in the "sounds" folder.

SOUND DELAY: the audio output is now kept awake with an inaudible looping buffer (Bloop then shows in the Windows volume mixer),
so a sound starts at once instead of a moment after the bubble appears. Switch "Sound effects" off in Settings -> Look to release it.

SETTINGS WITH PERSONALITY (0.17.0)
  - The logo at the top lets a bubble float up now and then. Click it: a burst of bubbles, and the island answers with a notification.
  - The highlight in the page list glides; the rows slide in one after another when you change page.
  - Switches stretch like a drop of water; every press sends a ripple; a slider shows its value in a bubble while you drag it.
  - Pick another theme and the colours fade into each other.

SETTINGS, REDESIGNED FROM SCRATCH (0.18.0)
  No more sidebar with rows. The window is a playground: the live island on top, a dock of round page buttons under it (each page has its
  own colour), and the settings of the page below as things you touch: switches are chips that light up, sliders are dials you turn
  (drag around the ring), themes are colour orbs, actions are pills. Hover anything and the line under the dock explains it.
  Save / Discard appear at the top right when you change something.

0.18.1: sliders are boxes now (drag sideways; the filled part is the level); more space between the little demo buttons and the dock.

0.19.0: every on/off setting is a card with a switch (name, a short line, the switch); the choices under "Maximized window" are cards with a round dot.

0.19.1: a tidy two-column grid (cards and sliders line up); the whole page scrolls smoothly under the top bar; lighter animations (only the changed rows are redrawn and sent to the screen, the theme fade is cheaper).

0.19.2: changing page / theme is smooth (the soft glow in the background is worked out ~5x faster, the first frame of a page no longer stutters); the black areas after opening or resizing the window are fixed; the log now says whether Do Not Disturb is on when a notification arrives.

0.19.3: "Show a test notification" is shown even while Do Not Disturb is on (it only tests the look); real notifications follow "Respect Do Not Disturb".

0.20.0
  - Bluetooth devices: when one connects (or disconnects) a pill appears with its picture (speaker, headphones, controller, keyboard, mouse,
    phone, watch) and its name, like the iPhone. Settings -> Notifications -> "Bluetooth device pills".
  - Only in Bloop: while Bloop runs, Windows' own notification pop-ups (banners) are switched off for your apps, so a notification shows
    in the island and not twice. Settings -> Notifications -> "Only show them in Bloop". They are put back when Bloop quits (and if Bloop
    could not tidy up, the next time it starts: the old values are kept in %APPDATA%\DynamicIsland\banner_backup.txt).

0.20.1: gaps in a song (empty lyric lines) no longer show a note symbol - only a little space; the small pill goes back to normal during a gap.

0.20.2 - lyrics in sync:
  - the song position is corrected as soon as it is more than 0.25 s off (before: 1.5 s), so the lines no longer drift behind the song;
  - timed lyrics that belong to a recording of another length (live version, remix) are no longer trusted: plain words are shown instead;
  - Settings -> Music -> "Lyrics timing": move the words earlier or later (-3.0 s ... +3.0 s) if one particular song is off;
  - the note marker for gaps in a song is back.

0.21.0
  - The device pill: the picture now FLOATS (a soft glow behind it, it rises and sinks slowly, its shadow breathes), like the little AirPods
    on the iPhone. New picture for earbuds (AirPods, Galaxy Buds ...): two small buds side by side.
  - Lettering: letters are placed with sub-pixel steps (even spacing instead of snapping to whole pixels), and light text is a touch
    stronger, so the notification text looks crisp like in a browser.

0.21.1: the device picture is smaller (36 instead of 54) and the pill is lower (56 instead of 66, 300 wide).

0.21.2: full-screen detection is stricter. The island hides for full-screen apps only when the window in front really covers the whole
screen without a title bar (a video, F11, a borderless game) or Windows reports exclusive full screen (a game). Windows' other "busy"
flags (a live wallpaper, presentation settings) no longer make it disappear. Want it always visible? Settings -> Windows ->
"Hide for full-screen apps" off (then Save).

0.21.3: lyrics - at the start of a song the first three lines are in plain view from the very first one (it used to hide the first line in the fade at the top edge).

0.22.0 - lyrics:
  - the lines at the bottom are clearer too (the fade at both ends is smaller, there is room under the last line);
  - Settings -> Music -> "How the lyrics move": Smooth (glide, the lit line grows), Instant (quick jumps, no frills), Focus (the lit
    line pops, the rest fades back), Karaoke (the words light up as they are sung). Try them in the live preview (open the player,
    press the lyrics button), then Save.

0.22.1
- Sliders in Settings no longer jump to where you click: pressing does nothing, and the value follows how far you drag.
- Karaoke lyrics: the bright part now glides smoothly (soft edge, 60 fps) instead of jumping letter by letter,
  the lit line grows and the other lines get smaller, and the timing is fitted to how long a line takes to sing.

0.22.2
- Lyrics timing now starts at -1.0 s (the setting that fits best).
- Lyric styles are down to two: Smooth and Karaoke.

0.23.0
- New app icon: a white pill with coral bubbles rising from it (bloop, bloop, bloop).
- The "Lyrics timing" slider is gone: the lyrics are always shifted by -1.0 s, which fits best.

0.23.1
- App icon: the pill that lets go of a drop, in cream, orange and black.

0.23.2
- "Install from a file" (Settings -> Updates) is hidden for everybody else: it only shows when the file
  %APPDATA%\DynamicIsland\developer.txt exists. CREATE THAT FILE (empty is fine) BEFORE you install this version,
  or you will not see the button any more.
- The Updates page is tidier: the switch, then the version with "Check now".
- A website for GitHub Pages is in the source (docs/).
