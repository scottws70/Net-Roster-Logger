## To Install:
<ul>
  <li>Click "Releases" on the right.</li>
  <li>Download your preferred package.</li>
  <li>Install (Or Run, in the case of the AppImage)</li>
  <li>Android (phones and tablets, including Amazon Fire): download the .apk on the device, open it, and allow installing from that app when asked.</li>
  <li>Go to Settings, set up your preferences and populate/import your roster.</li>
</ul>

<p>The Windows version will warn you that it's dangerous to install this. That's because it's unsigned. In order to make it signed and trusted, I would have to pay Microsoft. That's not something I do. When you see the pop up that tells you it blocked from installing and warning you, click More Info, then Run Anyway and it will install and work just fine.

# N8VCL Net Roster Logger — User Manual

Updated Oct 20, 2026 for v1.30.1 · @Scott

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
3. Go to the **Roster** tab and add stations one at a time with **+ Add Station**, or bring in a list at once with **Import Roster CSV** (see [Roster Tab](#roster-tab)).
4. Go to **Main** to start roll call. See [Main Tab (Roll Call)](#main-tab-roll-call).

Nothing here needs to be redone before every net — the Roster, Preamble, and Net Info are saved and stay in place until you change them.

# Overview of the Interface, Tabs, and Features:
## The Header (Appearance)

The header stays fixed at the top of the screen while everything below it scrolls.

- **Net Name** (top left) is a dropdown — click it to switch between nets, or choose **+ New Net…** to create one. See [Settings](#settings) → Net Profile.
- **NCS:** shows the configured NCS callsign.
- **Local Time** and **UTC Time** each show today's date above the current time, updated every second. Local time can be shown as 12- or 24-hour (Settings → Display); UTC is always 24-hour.
- **ID Timer** counts down from 10 minutes. **Start** begins the countdown; **Stop** stops it and resets it to 10:00. At zero, a beep sounds and a "Time to ID" pop-up appears, reminding you to identify and check for waiting traffic — click **OK** to restart the countdown. The whole ID Timer box can be hidden if you don't want it (Settings → Display).
- **Small screens** (tablets, phones, or a small window): the header shrinks to one short row — the clocks sit side by side without their dates, the ID Timer's Start/Stop sit beside it, and the tabs become a single row you swipe sideways.
- **Weather tiles** (installed app only), between the Net Name and NCS: Temp, Humidity, Wind, Barometer, and Precip from a weather station of your choice, refreshed every 5 minutes. Hover over a tile to see the station and when the reading was taken. The tiles dim when the reading is out of date or the last update failed. Set them up in Settings → [Weather](#weather).

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

For stations that check in before or outside the normal roll call. At the top is the call to read on the air: **"Any Mobiles, Portables, or Low Power stations?"** Fields: Callsign, Name, City, State.

- **Callsign** accepts the full callsign (`W1AW`) or just the suffix (`AW`), as long as that suffix matches exactly one station on the Roster. If it matches more than one, you'll be asked to type the full callsign instead.
- **Name lookup:** if you don't have the callsign handy, leave Callsign blank, type the station's **Name**, **Last Name**, or both (`Mark`, `Smith`, or `Mark Smith`), and press **Enter**. If exactly one station on the Roster matches, it's checked in. If several do, you'll see their callsigns and be asked to type the callsign instead. Each name must match the whole field (capitalization doesn't matter) — `Mar` won't find "Mark".
- A **known** callsign is checked in immediately.
- An **unknown** callsign prompts “Add to Roster?” — choosing Yes adds it using the Name/City/State you typed and checks it in; No just clears the fields. If it's a new station, and they want to be added to the roster, click "No" first, then re-enter the call, get the name, city, state, press enter, and answer Yes to "Add to Roster?" With a [QRZ login](#qrzcom-login) saved, the new station's QRZ photo is fetched in the background, and its name, email, and address are filled in from QRZ as described under [QRZ Photos, Names, Addresses, and Email](#qrz-photos-names-addresses-and-email).

## Main Tab (Roll Call)

Main shows one station at a time from the Roster, sorted by callsign suffix (the letters after the number — W1AW sorts under "AW"). The top line of the card reads the way you'd call the station, in large type: Callsign, Name, [Location](#roster-tab), and State — for example **WD0DMO Henry Holyrood KS**. Below that are Name, Last Name, and Spouse; then Birthday, Anniversary, and Last Check-in; then a Notes field and an Inactive checkbox. The station's [QRZ photo](#qrz-photos-names-addresses-and-email) sits at the top right; a station without one shows an empty frame there, so the text is always in the same place from card to card.

**Keyboard shortcuts** (These work as soon as you're on the Main tab, as long as you haven't clicked into the Notes field). They're also listed at the bottom of the card, and each one can be clicked instead of pressing the key:

| Key | Action |
| --- | --- |
| Space | Check the station in and move to the next one |
| → | Skip to the next station, without checking in |
| ← | Go back one station (works repeatedly, even past ones already checked in — use it to catch a missed or misheard callsign) |

**Touch screens** (Android tablets and phones, touch-screen laptops) can use gestures on the card instead: **double-tap** to check in and move to the next station, **swipe left** to skip, and **swipe right** to go back. A short message at the top confirms what happened ("✓ K0BHL checked in"). Touches that start on the Notes box, the Inactive checkbox, or the buttons are ignored. On small screens the card itself is more compact, so more of it fits without scrolling.

A station already checked in today is skipped automatically as you move forward. The roll call restarts from the top of the Roster each new day, so anyone not checked in today — whether that's from yesterday, last week, or never — will come up again.

**Notes** is a free-text note that stays with the station: whatever you type is saved as you type it and shows up again on their card at every net until you change or clear it — handy for "new grandson," "away until November," or anything you want to ask about next time. It's kept whether you check the station in or skip them. The field isn't auto-focused, so typing in it doesn't accidentally eat a Space or arrow key press — click into it first, then click outside it before using Space or the arrows again.

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

Checked in someone by mistake? Click the **✕** on their entry and confirm. The check-in is removed from today's log, any QTC it added is taken back off the day's count, and the station's Last Check-in (and Inactive setting, if it had one) go back to what they were. On a Roll Call net they'll come up again when roll call reaches them; if roll call has already passed them, use ← on the Main tab to go back.

Drag the panel's left edge to resize it; the text scales up or down with the width. The size you choose is remembered separately for each net.

## Reports Tab

- **Today** — how many stations checked in today, and today's QTC count. On a [Traffic Net](#traffic-net-mode), it also shows a one-line net report ready to send.
- **Net Report** — one row per net session for a chosen month: Date, Day, NCS, Check-ins, Traffic, Time (QTR), the stations that checked in from each area/region, and the traffic To and From each Area (or each group, on a net with no Area groups, such as one divided by state), with a totals row at the bottom. Pick a **Month**, and use **Days** to show only one day of the week (e.g. Fridays). **Export CSV** saves the report, ready to open in Excel, LibreOffice, or Zoho. Area/region columns appear on a [Traffic Net](#traffic-net-mode) with Traffic Groups set up.
- **This Month** — total check-ins and QTC for the current calendar month, plus **Net Sessions**: the number of distinct days this month with at least one check-in — a stand-in for how many times the net actually ran. Check-ins counts every check-in, so a station that checked in on 4 nets this month counts 4 times.
- **Stale Check-ins (>30 / >90 days)** — lists every station whose Last Check-in is older than that many days (or who has never checked in), sorted most-overdue first. Each row has a **✕** to remove that one station from the Roster, or use **Remove All Listed** to clear the whole batch at once. **Mark All Listed Inactive** is the gentler option: it hides them all from the Main roll call without deleting them, and each one comes back automatically the next time it checks in (stations already Inactive are marked "(inactive)" in the list). All three ask for confirmation first.
- **Backup All Data** / **Restore All Data** — see [Backup & Restore and Data Safety](#backup-restore-and-data-safety).

## Settings Tab

### Net Profile

Switch, create, or delete nets. Each net has its own Roster, check-in history, and settings — useful for running more than one net (e.g. a daily net and a weekly one, or a Morning net and Evening net, etc) from the same app. **+ New Net** creates a blank one; **Delete This Net** removes it and its data permanently (you can't delete your only net). The same switcher is also available as the Net Name dropdown in the header.

### Net Type

**Roll Call Net** (the default) or **Traffic Net**. Set separately for each net. See [Traffic Net Mode](#traffic-net-mode) for what changes.

### Net Info

Net Name and NCS Callsign, shown in the header.

### Preamble Script

The text shown on the [Preamble](#preamble) tab.

### Display

- **Color Theme** — Default (dark), High Visibility (black and yellow, for bright rooms or low-vision readability), Light, Blue, or Terminal (green text on black, in a monospace font, like an old green-screen terminal).
- **Local Time Format** — 12- or 24-hour. UTC is always 24-hour.
- **Show the 10-minute ID timer** — uncheck to hide the ID Timer box and turn off its reminder and beep entirely.

### Weather

Weather tiles for the top of the screen, set separately for each net.

- **Source:**
  - **Weather Underground station** — any station on wunderground.com, such as your own, by its Station ID (e.g. `KNELINCO123`). Precip is the total **since midnight**. Needs a Weather Underground **API key**, which WU gives free to owners of stations that upload to it (on wunderground.com, under your account's API Keys page). Paste it in **Weather Underground API Key** and click **Save Key**; like the QRZ login, it's saved only on this computer and never included in Backups.
  - **National Weather Service station** — an official observation station, usually an airport (e.g. `KLNK` Lincoln, `KOMA` Omaha, `KGRI` Grand Island). No key needed. Precip is the total for the **last 24 hours**. NWS stations report about once an hour.
  - **Off** hides the tiles.
- **Station ID** — the station to show.
- **Temp / Humidity / Wind / Barometer / Precip** — choose which tiles appear.
- **Test** fetches the weather right away and reports what it got (or why it couldn't).

Wind shows direction, speed, and gust, e.g. `NW 7 G14 mph`, or `Calm`. Barometer is in inches of mercury.

### File Locations

Choose a folder for Backups and a folder for Roster exports, so they save automatically instead of prompting each time.

- **Reset** clears the choice. This setting belongs to the computer, not to a net, so it isn't included in Backup/Restore.
- The location is a setting that applies to all nets. All other settings are per-net.
- On Android, folders can't be chosen: each Backup and Roster export opens Android's own save screen so you can pick where it goes.

### QRZ.com Login

Your QRZ.com username and password, used to look stations up on QRZ for their photos and names (see [QRZ Photos and Names](#qrz-photos-names-addresses-and-email)). Photos need an active QRZ **XML Logbook Data** subscription; a free QRZ account can sign in but gets only limited data back.

- **Save** stores the login. To change just the username, leave Password blank and the saved password is kept.
- **Test Login** signs in to QRZ and tells you whether the account's XML subscription is active, and until when.
- **Clear** removes the saved login.

Like File Locations, the login belongs to the computer, not to a net: it's saved only in the app's settings folder on this computer, applies to all nets, and is never included in Backups. This section works only in the installed app.

### Sharing

Use your main computer's nets from a tablet, phone, or laptop on the same network, or anywhere over Tailscale. One device at a time is in charge: whatever you do on one shows up on the others within a couple of seconds, so you can switch devices mid-net.

**On the main computer** (Windows or Linux app):

1. Tick **Share this computer's nets**. The section then shows the address to use (such as `http://192.168.1.20:7373`; Tailscale addresses start with 100) and a 6-digit **pairing code**.
2. Leave Net Logger open on that computer; the other devices work on its data. Sharing turns itself back on the next time the app starts.
3. On Windows, allow Net Logger through the firewall when asked (Private networks).

**On the other device**, either:

- open the address in any web browser (Chrome, Safari, the Fire tablet's Silk) — nothing to install — or
- in the Net Logger app (such as the Android app), go to **Settings → Sharing**, enter the main computer's address (just `192.168.1.20` or its Tailscale name is enough) and tap **Connect**.

Enter the pairing code once; that device then goes straight in from then on.

- QRZ lookups, photos, and weather on the other device use the main computer's QRZ login and weather key, so they don't need to be set up there.
- If the main computer can't be reached, a red bar says so; changes made meanwhile are sent when it's back.
- **Back to This Device's Nets** (in the app) returns to the device's own data, which is kept separately and untouched. **Disconnect** also forgets the pairing.
- **New Code** makes a new pairing code; every device will need to enter it again. Use it if someone you don't want on your nets has the code.
- If two people change things on two devices at the same moment, the later save wins.

## Roster Tab

The last tab, right after Settings. Each net has its own Roster; it's shown on both Roll Call and Traffic nets.

Each station is shown as a card: Callsign, Name, Last Name, Spouse, City, Location, State on the first line; Birthday, Anniversary, Last Check-in, QRZ URL, and an Inactive checkbox on the second. **+ Add Station** adds a blank one. Callsign fields force uppercase as you type, and a slashed zero (`Ø`, as many logs write it) is turned into a real `0` — in the Roster, in every check-in box, and in imported CSVs — so callsigns sort by suffix and look up correctly on QRZ.

**Location** is how the net refers to where a station is — "East Campus, Lincoln", "N of McCook", "NE City" — and it's what the Main card shows. It started out as each station's City (or, for a station whose city a QRZ lookup had replaced, the city it was listed under before), and new stations start with the City you type. Edit it freely: QRZ lookups never change it. If it's left blank, the City is shown instead.

A third line holds Area, Region, Phone, Email, Street, and ZIP. Area and Region are what a [Traffic Net](#traffic-net-mode) uses to place each station in its group (e.g. Area `Eastern`, or Region `5`).

**Import Roster CSV** adds or updates stations in bulk. Recognized columns, in any order, case-insensitive: Callsign (or Call Sign), Name, Last Name (or Last, Surname), Spouse, City, Location, State, URL, Birthday, Anniversary, Last Check-in, Inactive (Yes/No), Area, Region, Phone, Email, Street (or Address), ZIP, Notes. A **First** column fills Name when there's no Name column. Spreadsheet leftovers are cleaned up on the way in: a Region of `5.0` becomes `5`, and a ZIP that lost its leading zero (`6484`) gets it back (`06484`). A row matching an existing callsign updates it (a blank cell leaves that field as it was); other rows are added as new stations. Birthday and Anniversary accept the same flexible formats described under [Birthdays & Anniversaries](#birthdays-anniversaries).

**Export Roster CSV**, next to Import, saves the current Roster with every field above, in the same format Import accepts, so it round-trips cleanly.

### QRZ Photos, Names, Addresses, and Email

With a [QRZ login](#qrzcom-login) saved (installed app only), the app can look stations up on QRZ.com:

- **Get from QRZ** on a station's card looks that one station up. It downloads a fresh copy of the station's QRZ photo and shows it on the card and on the Main roll-call card, and fills in **Name**, **Last Name**, **Email**, and the address where they're blank. Names and email you've typed are never overwritten. QRZ's nickname is used for Name when the station has one, middle initials are dropped, and all-caps license names and addresses are tidied up (`HIRAM P` → `Hiram`, `RR 1 BOX 5` → `RR 1 Box 5`, `MCCOOK` → `McCook`).
- **Addresses:** a station with **no Street** on file gets QRZ's whole mailing address (Street, City, State, ZIP). The City you had typed — often an informal one like "N of McCook" or "NE City" — stays in the station's **Location**, so the Main card still reads the way the net knows them. If the State you had was different, it's kept in the station's [Notes](#main-tab-roll-call) as `Listed as: N of McCook, NE`. Spelling differences in spacing or punctuation don't count ("Springhill" and "Spring Hill" are the same). A station that already has a Street keeps its address; only its blank parts are filled, and only when its street and city match QRZ's.
- **Fetch Missing from QRZ** looks up every station that hasn't been looked up yet (stations looked up by an earlier version of the app that filled in fewer fields count as not yet) and is missing a photo, a name, an email, or part of its address, one at a time with a short pause between, so QRZ isn't flooded. It shows its progress and a summary at the end; click **Stop** to end it early. It reuses photos already saved rather than downloading them again. A station QRZ has no photo for isn't asked about again; use **Get from QRZ** on its card to retry. Each net has its own Roster, so run it once in each net.
- **New stations are looked up automatically:** when you answer Yes to “Add to Roster?” from Early Check-ins, the Out of Sequence box, or End of the List, or type a callsign into a station card.

Photos are downloaded once and saved on this computer, so they show during nets with no internet connection. They aren't included in Backups or roster CSV exports; after restoring a backup on another computer, Fetch Missing from QRZ downloads them again.

## Traffic Net Mode

For nets whose purpose is passing formal traffic rather than calling a roster. Turn it on in Settings → Net Type → **Traffic Net**; it applies only to that net, so a Roll Call net and a Traffic net can live side by side.

**What changes:**

- **Tabs:** only **Preamble**, **Main**, **Reports**, and **Settings** are shown. Birthdays & Anniversaries, Traffic, End of the List, and the roll-call Main tab are hidden.
- **Main** is the Early Check-ins screen, renamed, with the fields **Callsign**, **Name**, and **State** (City is hidden), plus the traffic fields described below. Suffix matching, name lookup, and the “Add to Roster?” prompt work the same as in [Early Check-ins](#early-check-ins). Press **Enter** from any field to check the station in.
- **Reports → Today** adds a one-line net report you can copy and send, in the form:

  `<Net Name> net report for <Mon D> utc <check-ins> <QTC> <callsigns…> <NCS>/NCS`

  for example `BTN net report for Oct 3 utc 12 4 W1AW K8XYZ … N8VCL/NCS`. The date is in UTC, and the NCS callsign is always listed last with `/NCS` — the session's NCS if one was entered (see below), otherwise the one from Net Info.

Switching a net back to Roll Call Net restores all the tabs; nothing is lost either way.

### Net Session

At the top of Main is the **Net Session** box for today's net:

- **Start Net** and **Secure Net** stamp the current time into **Time Started** and **Time Secured**. Either time can also be typed or corrected by hand.
- **NCS** is filled in from Net Info; change it if someone else is running tonight's net.
- **QNI** (check-ins), **QTC** (messages listed), and **QTR** (how long the net ran, Secured minus Started) update as you go.

These carry into the [Net Report](#reports-tab), one row per session.

### Traffic Groups

Settings → **Traffic Groups** is the list of areas, regions, or states this net sorts stations into and logs traffic to — for an NTS Area net, something like EAN, R5, R9, R10, WAN; for a net that covers a single region, one group per state (or per few states). Each group has:

- **Label** — the short name shown on screen and in reports (`EAN`, `R5`).
- **Type** — **Area**, **Region**, or **State**.
- **Matches roster** — the Area, Region, or State value on the Roster that puts a station in this group (`Eastern`, `5`, `NE`). List several values separated by commas to cover more than one — e.g. a State group matching `IA, MO`.

The most specific match wins: a station's **State** is checked first, then its **Region**, then its **Area** — so a Region 8 station whose Area is Eastern lands in EAN when there's no R8 group, and a state-divided net can add a Region group as a catch-all for stations from states it doesn't list. Capitalization and extras like "Area" or "Region" don't matter (`EASTERN`, `Eastern Area`, and `Eastern` all match). Use ▲ ▼ to set the order the groups appear in, and **✕** to remove one.

**Use NTS Central Area preset** fills in EAN (Area: Eastern), R5, R9, R10 (Regions 5, 9, 10), and WAN (Area: Western) in one click.

With **no** groups set up, Main keeps the simple single **QTC** and **Destination** fields, and the panel shows `QTC - 2 Columbus` or `QRU`.

### Checking In With Traffic

With Traffic Groups set up, the Main check-in form has:

- **Group** — filled in automatically from the Roster when you leave the Callsign field; change it if needed. A new station added with “Add to Roster?” keeps the group you picked as its Area or Region.
- **Traffic listed, by destination** — one box per group. Enter how many messages the station has for each place, or leave them blank for QRU. The total is added to the day's QTC.

The **Check-ins Today** panel shows each station as Callsign, then Name - State · Group, then `QTC - R9 2, WAN 1` (a ✓ marks traffic that's been sent) or `QRU`.

### Traffic Board

Below the check-in form, the **Traffic Board** lists today's check-ins grouped by area/region, with a column for each destination:

- Change a station's **Group** from its dropdown.
- Type in or correct the number of messages listed **To** each place; QTC updates to match.
- Tick **Sent** once that traffic has been passed. (QTC counts traffic *listed*, so Sent doesn't change it.)
- The **Totals** row shows how many messages are listed to each place, and how many of them have been sent.

## Backup & Restore and Data Safety

Everything — Roster, check-in history, settings — is stored locally. There's no cloud connection or separate server copy. Your data is YOURS.&#32;

**Backup All Data**, on the [Reports](#reports) tab, downloads a dated JSON snapshot named `[Net Name]-Backup-[date].json`; **Restore All Data** loads one back in, replacing everything currently in that net.

Back up <em><strong>frequently. At least once a week.</em></strong>

**Recommended habit:** keep one fixed filename/location as your working copy rather than a new file per version, and back up right after every net.

If you need someone to substitute for you and they are running N8VCL Roster Net Logger, you can send the backup JSON  and have them Restore to get all the settings you have, or you can Export Roster CSV and send that and they will have the most up-to-date roster, while all other settings remain default.

# Recommended Use:

I recommend first setting your net up in the Settings tab, and once that's done and you're ready to call a net, just work left to right across the tabs. Click Preamble and read, click Birthdays and Anniversaries and call them out. Click Traffic and ask if there are any stations with Traffic for the net and record it here. Click Early Check-ins and ask for those folks who have to get in-and-out. Click Main and go through the Roll Call. Etc. It was designed to work in that order. Hopefully it's a logical workflow to you.

## Troubleshooting

**A station I expect to see on Main isn't showing up.** Check whether it's marked Inactive in the Roster tab — inactive stations are hidden from roll call until they check in again from any tab.  Or perhaps they checked in early.

**“Matches multiple callsigns” message in Early Check-ins or End of the List.** The suffix you typed matches more than one station. Type the full callsign instead.

**“No roster match” when checking in by name.** Name lookup needs the whole Name, the whole Last Name, or both, exactly as entered on the Roster (capitalization aside). Enter the callsign instead, or check the spelling in the Roster tab.

**A station shows up as Unassigned on the Traffic Board.** Its Roster Area/Region doesn't match any Traffic Group. Fill in its Area or Region in the Roster tab, or check the group's "Matches roster" value.

**Some tabs are missing.** The net is probably set to Traffic Net — see Settings → Net Type and [Traffic Net Mode](#traffic-net-mode).

**A CSV import didn't pick up a Birthday or Anniversary correctly.** Check the format against [Birthdays & Anniversaries](#birthdays-anniversaries) — a bare year with no day (`1984`) is stored as year-only on purpose, so it won't show as a match.
