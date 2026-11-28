# Changelog

All notable changes to Bloop. Newest first. Versions are the tags of the GitHub releases.

## 0.25.1 - eyes that follow
- The little face's eyes follow your mouse wherever it is on the screen (and look up or down too). A moving mouse also wakes the face
  when it has nodded off. While it sleeps the island only looks at the mouse about twice a second.

## 0.25.0 - who is listening, what is arriving
- **Microphone and camera dot:** a small dot beside the island (orange: microphone, green: camera) while a program uses them, and a pill
  with the program's name when one starts. It reads the list Windows keeps for its own privacy settings. Settings → Notifications.
- **Download pill:** while a browser downloads a file into your Downloads folder (Chrome, Edge, Firefox, Opera ...), a pill shows its name
  and size so far, with a light running along a track (the total size is not known, so there is no percentage), and says *Downloaded* when
  it is done. A message always takes over the island at once. Settings → Notifications.

## 0.24.0 - a pill for copying, and a face
- **Copied pill:** when you copy something, a small pill says *Copied* and what kind of thing it was (Text, Image or Files). It never
  shows or stores what was copied, and it stays quiet when the program asks not to be recorded (password managers do).
  Settings → Notifications → *Copied pill*.
- **Pill face:** while nothing is playing the resting pill has a little face made of pills. It blinks, glances around, smiles for a moment
  after a notification and nods off after a minute and a half of quiet (then nothing runs at all). Settings → Look → *Pill face*.

## 0.23.x
- **0.23.2** The Updates page is tidier: the switch, then the version with *Check now*. Added the website (`docs/`) and the developer-only
  *Install from a file* option (it appears only when `%APPDATA%\DynamicIsland\developer.txt` exists).
- **0.23.1** New app icon: a pill that lets go of a drop, in cream, orange and black.
- **0.23.0** The lyrics are always shifted by -1.0 s (the best fit found), so the *Lyrics timing* slider is gone.

## 0.22.x - lyrics
- **0.22.2** Two lyric styles remain: *Smooth* and *Karaoke*.
- **0.22.1** Sliders in Settings no longer jump to where you click; they follow how far you drag. Karaoke lyrics glide smoothly instead of
  jumping letter by letter, the lit line grows, and the timing follows how long a line takes to sing.
- **0.22.0** The lines at the bottom of the lyrics are clearer; four lyric styles to choose from (reduced to two in 0.22.2).

## 0.21.x - details
- **0.21.3** At the start of a song the first lines are in plain view from the very first one.
- **0.21.2** Stricter full-screen detection: the island hides only for a window that really covers the whole screen without a title bar
  (video, F11, borderless game) or for exclusive full screen. Settings → Windows → *Hide for full-screen apps* turns it off.
- **0.21.1** Smaller device picture and a lower device pill.
- **0.21.0** The Bluetooth device picture floats (soft glow, slow rise and fall), new earbuds picture, crisper lettering
  (sub-pixel letter placement).

## 0.20.x - devices and notifications
- **0.20.2** Lyrics in sync: the song position is corrected as soon as it is 0.25 s off; timed lyrics of a recording with another length
  (live version, remix) are no longer trusted.
- **0.20.1** Gaps in a song show a little space instead of a note symbol (the marker came back in 0.20.2).
- **0.20.0** Bluetooth device pills with the device's picture (speaker, headphones, controller, keyboard, mouse, phone, watch).
  *Only show them in Bloop*: Windows' own notification pop-ups are switched off while Bloop runs, and restored when it quits.

## 0.19.x - the settings window, tidied
- **0.19.3** *Show a test notification* ignores Do Not Disturb (it only tests the look).
- **0.19.2** Page and theme changes are smooth; black areas after opening or resizing the window are fixed.
- **0.19.1** A tidy two-column grid; only the changed parts of the window are redrawn.
- **0.19.0** Every on/off setting is a card with a switch; the choices under *Maximized window* are cards with a round dot.

## 0.18.x - redesign
- The settings window rebuilt from scratch: the live island on top, a dock of round page buttons, settings as things you touch
  (switches, box sliders, colour orbs). Save / Discard appear when something changed.

## 0.17.x and earlier
- A logo that lets bubbles float up, a gliding page highlight, stretchy switches, sliders with a value bubble, themes that fade into
  each other, per-status sound effects, home view with a notification log, self-update from GitHub Releases, system tray icon.
- Bloop began as an Electron app and was rewritten in Rust (no web engine) with the same look.
