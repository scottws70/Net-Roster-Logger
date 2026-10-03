## To Install:
<ul>
  <li>Click "Releases" on the right.</li>
  <li>Download your preferred package.</li>
  <li>Install (Or Run, in the case of the AppImage)</li>
  <li>Go to Settings, set up your preferences and populate/import your roster.</li>
</ul>

<p>The Windows version will warn you that it's dangerous to install this. That's because it's unsigned. In order to make it signed and trusted, I would have to pay Microsoft. That's not something I do. When you see the pop up that tells you it blocked from installing and warning you, click More Info, then Run Anyway and it will install and work just fine.

# N8VCL Net Roster Logger — User Manual

Updated Oct 3, 2026 for v1.29.1 · @Scott

## Overview

Net Logger is a self-contained app for a ham radio net control station to run a Roll-Call based net: track who checks in, log traffic (QTC), flag birthdays and anniversaries, and keep the roster up to date, all from one screen.

It runs in a single window, with no internet connection required. Roster, check-in history, and settings are all stored locally.

The app can run more than one net, each with its own Roster and history and other settings. (see [Settings](#settings) → Net Profile).

Each net can be set up as a **Roll Call Net** (the default, described throughout this manual) or a **Traffic Net**, which simplifies the screens for formal traffic handling. See [Traffic Net Mode](#traffic-net-mode).

## Quick Start:

1. Open the app.
2. Go to **Settings** and fill in:
   - **Net Name** and **NCS Callsign**, under Net Info.
   - The **Preamble** script read at the start of the net.
   - The **Roster**: add stations one at a time with **+ Add Station**, or bring in a list at once with **Import Roster CSV** (see [Settings](#settings) → Roster).
3. Go to **Main** to start roll call. See [Main Tab (Roll Call)](#main-tab-roll-call).

Nothing here needs to be redone before every net — the Roster, Preamble, and Net Info are saved and stay in place until you change them.

# Overview of the Interface, Tabs, and Features:
## The Header (Appearance)

The header stays fixed at the top of the screen while everything below it scrolls.

- **Net Name** (top left) is a dropdown — click it to switch between nets, or choose **+ New Net…** to create one. See [Settings](#settings) → Net Profile.
- **NCS:** shows the configured NCS callsign.
- **Local Time** and **UTC Time** each show today's date above the current time, updated every second. Local time can be shown as 12- or 24-hour (Settings → Display); UTC is always 24-hour.
- **ID Timer** counts down from 10 minutes. **Start** begins the countdown; **Stop** stops it and resets it to 10:00. At zero, a beep sounds and a "Time to ID" pop-up appears, reminding you to identify and check for waiting traffic — click **OK** to restart the countdown. The whole ID Timer box can be hidden if you don't want it (Settings → Display).

## Preamble Tab

Displays the script you read to open the net, exactly as typed into Settings → Preamble Script. It's read-only on this tab — edit the text in Settings.

## Birthdays & Anniversaries Tab

Lists every station whose Birthday or Anniversary matches today's month and day, with Callsign, Name, Spouse, and which occasion it is.

Birthday and Anniversary fields don't need a full date. In the Roster, you can enter:

- **A full date** (e.g. `1984-03-22`) — matched every year on that day.
- **Month and day only** (e.g. `3/22`) — also matched every year; used when the year isn't known.
- **A year only** (e.g. `1984`) — kept on file, but since the day isn't known, it will never trigger a match here.

## Traffic Tab

Logs QTC (formal traffic) as a running list of rows: Callsign, Destination, Qty, and Sent?.

Fill in a row, then press **Enter** or **Tab** out of it to open a new blank row automatically. Setting **Sent?** to **Yes** adds 1 to that day's QTC count on the [Reports](#reports) tab; switching it back off subtracts it again, so a row can be corrected without double-counting. If the ID-Timer is enabled, the pop-up will remind you "is there any traffic waiting" so you can ID and then remind the net if you're holding traffic and where it's going.

## Early Check-ins Tab

For stations that check in before or outside the normal roll call. Fields: Callsign, Name, City, State.

- **Callsign** accepts the full callsign (`W1AW`) or just the suffix (`AW`), as long as that suffix matches exactly one station on the Roster. If it matches more than one, you'll be asked to type the full callsign instead.
- **Name lookup:** if you don't have the callsign handy, leave Callsign blank, type the station's **Name**, and press **Enter**. If exactly one station on the Roster has that name, it's checked in. If several do, you'll see their callsigns and be asked to type the callsign instead. The name must match the whole Name field (capitalization doesn't matter) — `Mark` finds "Mark", but not "Mark Smith".
- A **known** callsign is checked in immediately.
- An **unknown** callsign prompts “Add to Roster?” — choosing Yes adds it using the Name/City/State you typed and checks it in; No just clears the fields. If it's a new station, and they want to be added to the roster, click "No" first, then re-enter the call, get the name, city, state, press enter, and answer Yes to "Add to Roster?"

## Main Tab (Roll Call)

Main shows one station at a time from the Roster, sorted by callsign suffix (the letters after the number — W1AW sorts under "AW"). Its card shows Name, Spouse, City, State, Birthday, Anniversary, Last Check-in, a Notes field, and an Inactive checkbox.

**Keyboard shortcuts** (These work as soon as you're on the Main tab, as long as you haven't clicked into the Notes field):

| Key | Action |
| --- | --- |
| Space | Check the station in and move to the next one |
| → | Skip to the next station, without checking in |
| ← | Go back one station (works repeatedly, even past ones already checked in — use it to catch a missed or misheard callsign) |

A station already checked in today is skipped automatically as you move forward. The roll call restarts from the top of the Roster each new day, so anyone not checked in today — whether that's from yesterday, last week, or never — will come up again.

**Notes** is a free-text field for that day's check-in; it isn't auto-focused, so typing in it doesn't accidentally eat a Space or arrow key press — click into it first.

**Inactive** hides a station from the roll call entirely (it won't come up with Space or →). The attribute is cleared automatically the next time that station checks in, from any tab, so they will once again appear during the Roll-Call.

When you reach the end of the Roster, Main switches automatically to **End of the List**.

### Out of Sequence, Early, or Late

Below the station card is a small check-in box for stations that break in during roll call — a "CONTACT," a late arrival, or "check in my wife too" — so you can log them without leaving the Main tab. It works exactly like [Early Check-ins](#early-check-ins): the same Callsign / Name / City / State fields, suffix matching, name lookup, and “Add to Roster?” prompt.

Anyone checked in this way is skipped automatically when roll call reaches them. While your cursor is in one of these fields, Space and the arrow keys type normally instead of moving the roll call; click anywhere outside the box to go back to using the shortcuts.

## End of the List Tab

Works exactly like [Early Check-ins](#early-check-ins) — same four fields, same suffix matching, same “Add to Roster?” prompt — for catching late, missed, or new stations.

The app switches here automatically the moment you reach the end of the Roster on the Main tab, whether by Space or →.

## Check-ins Today Panel (Appearance)

A panel on the right, visible on every tab, listing Callsign and Name for every station checked in today. It updates the moment a check-in happens — from Main, Early Check-ins, or End of the List — and auto-scrolls down to show each new entry as it's added.

Drag the panel's left edge to resize it; the text scales up or down with the width. The size you choose is remembered separately for each net.

## Reports Tab

- **Today** — how many stations checked in today, and today's QTC count. On a [Traffic Net](#traffic-net-mode), it also shows a one-line net report ready to send.
- **This Month** — total check-ins and QTC for the current calendar month, plus **Net Sessions**: the number of distinct days this month with at least one check-in — a stand-in for how many times the net actually ran. Check-ins counts every check-in, so a station that checked in on 4 nets this month counts 4 times.
- **Stale Check-ins (>30 / >90 days)** — lists every station whose Last Check-in is older than that many days (or who has never checked in), sorted most-overdue first. Each row has a **✕** to remove that one station from the Roster, or use **Remove All Listed** to clear the whole batch at once; both ask for confirmation first.
- **Export Roster CSV** — downloads the current Roster in the same format [Import Roster CSV](#settings) accepts, so it round-trips cleanly.
- **Backup All Data** / **Restore All Data** — see [Backup & Restore and Data Safety](#backup-restore-and-data-safety).

## Settings Tab

### Net Profile

Switch, create, or delete nets. Each net has its own Roster, check-in history, and settings — useful for running more than one net (e.g. a daily net and a weekly one, or a Morning net and Evening net, etc) from the same app. **+ New Net** creates a blank one; **Delete This Net** removes it and its data permanently (you can't delete your only net). The same switcher is also available as the Net Name dropdown in the header.

### Net Type

**Roll Call Net** (the default) or **Traffic Net**. Set separately for each net. See [Traffic Net Mode](#traffic-net-mode) for what changes.

### Net Info

Net Name and NCS Callsign, shown in the header.

### Display

- **Color Theme** — Default (dark), High Visibility (black and yellow, for bright rooms or low-vision readability), or Light.
- **Local Time Format** — 12- or 24-hour. UTC is always 24-hour.
- **Show the 10-minute ID timer** — uncheck to hide the ID Timer box and turn off its reminder and beep entirely.

### File Locations

Choose a folder for Backups and a folder for Roster exports, so they save automatically instead of prompting each time.

- **Reset** clears the choice. This setting belongs to the computer, not to a net, so it isn't included in Backup/Restore.
- The location is a setting that applies to all nets. All other settings are per-net.

### Preamble Script

The text shown on the [Preamble](#preamble) tab.

### Roster

Each station is shown as a two-line card: Callsign, Name, Spouse, City, State on the first line; Birthday, Anniversary, Last Check-in, QRZ URL, and an Inactive checkbox on the second. **+ Add Station** adds a blank one. Callsign fields force uppercase as you type.

**Import Roster CSV** adds or updates stations in bulk. Recognized columns, in any order, case-insensitive: Callsign, Name, Spouse, City, State, URL, Birthday, Anniversary, Last Check-in, Inactive (Yes/No). A row matching an existing callsign updates it; other rows are added as new stations. Birthday and Anniversary accept the same flexible formats described under [Birthdays & Anniversaries](#birthdays-anniversaries).

## Traffic Net Mode

For nets whose purpose is passing formal traffic rather than calling a roster. Turn it on in Settings → Net Type → **Traffic Net**; it applies only to that net, so a Roll Call net and a Traffic net can live side by side.

**What changes:**

- **Tabs:** only **Preamble**, **Main**, **Reports**, and **Settings** are shown. Birthdays & Anniversaries, Traffic, End of the List, and the roll-call Main tab are hidden.
- **Main** is the Early Check-ins screen, renamed, with the fields **Callsign**, **Name**, **State**, **QTC**, and **Destination** (City is hidden). Suffix matching, name lookup, and the “Add to Roster?” prompt work the same as in [Early Check-ins](#early-check-ins).
  - **QTC** is the number of messages the station is holding; leave it blank if they have none. It's added to the day's QTC count.
  - **Destination** is where their traffic is going.
  - Press **Enter** from any field to check the station in.
- **Check-ins Today panel** shows each station as Callsign, then Name - State, then either `QTC - 2 Columbus` (holding traffic, with destination) or `QRU` (nothing to pass).
- **Reports → Today** adds a one-line net report you can copy and send, in the form:

  `<Net Name> net report for <Mon D> utc <check-ins> <QTC> <callsigns…> <NCS>/NCS`

  for example `BTN net report for Oct 3 utc 12 4 W1AW K8XYZ … N8VCL/NCS`. The date is in UTC, and the NCS callsign (from Net Info) is always listed last with `/NCS`.

Switching a net back to Roll Call Net restores all the tabs; nothing is lost either way.

## Backup & Restore and Data Safety

Everything — Roster, check-in history, settings — is stored locally. There's no cloud connection or separate server copy. Your data is YOURS.&#32;

**Backup All Data**, on the [Reports](#reports) tab, downloads a dated JSON snapshot named `[Net Name]-Backup-[date].json`; **Restore All Data** loads one back in, replacing everything currently in that net.

Back up <em><strong>frequently. At least once a week.</em></strong>

**Recommended habit:** keep one fixed filename/location as your working copy rather than a new file per version, and back up right after every net.

If you need someone to substitute for you and they are running N8VCL Roster Net Logger, you can send the backup JSON  and have them Restore to get all the settings you have, or you can Export Roster CSV and send that and they will have the most up-to-date roster, while all other settings remain default.

# Recommended Use:

I recommend first setting your net up in the Settings tab, and once that's done and you're ready to call a net, just work left to right across the tabs. Click Preamble and read, click Birthdays and Anniversaries and call them out. Click Traffic and ask if there are any stations with Traffic for the net and record it here. Click Early Check-ins and ask for those folks who have to get in-and-out. Click Main and go through the Roll Call. Etc. It was designed to work in that order. Hopefully it's a logical workflow to you.

## Troubleshooting

**A station I expect to see on Main isn't showing up.** Check whether it's marked Inactive in Settings → Roster — inactive stations are hidden from roll call until they check in again from any tab.  Or perhaps they checked in early.

**“Matches multiple callsigns” message in Early Check-ins or End of the List.** The suffix you typed matches more than one station. Type the full callsign instead.

**“No roster match” when checking in by name.** Name lookup needs the whole Name exactly as it's entered on the Roster (capitalization aside). Enter the callsign instead, or check the spelling in Settings → Roster.

**Some tabs are missing.** The net is probably set to Traffic Net — see Settings → Net Type and [Traffic Net Mode](#traffic-net-mode).

**A CSV import didn't pick up a Birthday or Anniversary correctly.** Check the format against [Birthdays & Anniversaries](#birthdays-anniversaries) — a bare year with no day (`1984`) is stored as year-only on purpose, so it won't show as a match.
