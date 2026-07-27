# AE-5 Control — UI/UX review and redesign brief

Audience: the developer who will implement this. Written after inspecting
the running application against the private Sound Blaster Command captures
at `~/.local/share/ae5-control/windows-ui-reference/2026-07-26/`.

Scope note: the visual pass in `dd3cd8d` is mine, so part of what follows
criticises my own work.

## Verdict, stated fairly

The current design is **not incoherent**. It is a faithful clone of
Command's information architecture, which is a defensible choice for a user
migrating from Windows — recognition beats novelty when someone is already
frustrated. Three things in it are genuinely better than Command: live
hardware readback, explicit marking of unsupported features, and a legible
dark palette.

What it has is a **craft problem and a hierarchy problem**, not a concept
problem. Six specific defects, worst first.

### 1. Semantic collapse — different objects rendered identically

"Your profiles" is one row of visually identical cards holding three
unrelated kinds of thing:

| Card | What it actually is |
|---|---|
| Current hardware · 48 controls | live device **state**, not a profile |
| EQ · SHP Last, EQ · SHP9500 test | **EQ-only** presets, 12 controls |
| Jabra · Headphones | a **full** profile, 9 controls |

Applying the second changes ten EQ bands. Applying the third changes
routing and effects. The UI gives the user no way to know that. This is
the single worst defect: it makes a destructive action look identical to a
harmless one.

### 2. Engineer vocabulary in user surfaces

"48 controls read from the AE-5". "12 validated controls". "9 validated
controls". Control counts are an implementation detail. The user's question
is *what will this change, and is it on now* — never *how many ALSA
elements does it touch*.

### 3. Values without scale or unit

The Acoustic Engine shows bare integers: Surround 0, Crystalizer 50,
Bass 53, Smart Volume 15, Dialog+ 0. Fifty-three of what, out of what?
Worse, Dialog+ reads **toggle on, value 0** — a contradictory state shown
with no explanation. Command at least labels Smart Volume's poles
(Night/Loud).

### 4. Broken layout, not styled layout

"Adventure And Ac…" is truncated. The fifth built-in card is clipped
mid-word at the viewport edge. The Acoustic Engine dials are cut off at the
bottom. Horizontal rows overflow with no scroll affordance. These read as
bugs to a user, and they undermine trust in everything else on screen.

### 5. Thirty-three identical tiles

Every Command-default card says "Speaker + headphone variants" and
"Preview & apply". Thirty-three of them. Command carries this with
artwork; we deliberately cannot (Creative's imagery stays out of the
repo). Without artwork, a wall of identical tiles is *worse* than a list —
it spends maximum space to convey minimum information. The answer is
categories and search, not tiles.

### 6. Inverted information hierarchy

The most safety-critical fact on screen — **OUTPUT MUTED** — is rendered
as small footer caps at decoration weight, beside the vanity string
"LINUX NATIVE · WAYLAND". Meanwhile 33 interchangeable tiles get the
largest area. The page is proportioned inversely to what matters.

Also: three competing on/off idioms coexist (the header "Acoustic engine"
switch, per-effect switches, and "ACTIVE" as a text badge), and the sidebar
is nine flat items where Command groups them with separators.

## How I would build it from zero

The current IA is organised by **hardware subsystem** — Playback,
Recording, Mixer. That is the engineer's model of the card. The user's
model is a set of jobs:

1. *Is my audio working, and if not, why?*
2. *Make it sound right for what I'm doing now.*
3. *Change one specific thing.*
4. *Don't damage my hearing or my headphones.*

Command serves none of these directly, so cloning Command inherits the
mismatch. Four surfaces instead of nine pages:

### Now — the home surface

One screen answering "what is my card doing right now": active profile,
active route, the full gain chain, effects state, and any health warning
with a one-click guarded repair. This does not exist in Command and is the
most valuable thing we can build, because every complaint this project has
received has been a variant of *I can't tell what state my card is in*.

### Sound — profiles and effects together

One task, one surface. Profiles as a **searchable, grouped list**
(Games / Movies / Music / Communication), each showing its scope as a
sentence — "changes 10 EQ bands", "changes routing and 8 effects" — and,
after applying, a live diff: *3 controls differ from this profile*. That
diff is buildable today; the comparison logic already exists.

Effects as one compact row with real units and named poles, not five large
cards.

### Routing — outputs, inputs, layout

The Playback/Recording/Mixer merge, expressed as a signal path rather than
three subsystem pages.

### Advanced — the honest escape hatch

Raw control list, diagnostics, kernel and compatibility, lighting. Power
users get everything; the default path stays clean.

### Cross-cutting rules

- **A persistent output strip**, high contrast, always visible: route,
  volume, mute, gain stage. Safety-critical state gets safety-critical
  visual weight. Muted output must be impossible to miss.
- **Applied versus requested.** The backend already returns readback for
  every write. Show when they diverge instead of assuming success — this is
  our real advantage over Command and it is currently invisible.
- **Units always.** dB, Hz, %, never a bare integer.
- **One idiom per concept.** A switch is a switch everywhere.
- **No layout may clip.** Anything that can overflow scrolls, with a
  visible affordance.

## Handover list

Ordered by user-visible value per unit of effort. Items 1–4 are defect
repairs and should land before any restructuring.

| # | Change | Why | Size |
|---|---|---|---|
| 1 | Split "Your profiles" into *Live state* and *Saved profiles*; label each profile's scope in words | Removes the destructive/harmless ambiguity | S |
| 2 | Fix all clipping: truncated titles, cut-off fifth card, clipped dials; add scroll affordances | These read as bugs | S |
| 3 | Promote output state (route, volume, mute) into a high-contrast persistent strip | Safety-critical info currently whispered | S |
| 4 | Add units and named poles to every effect value; resolve the toggle-on/value-0 contradiction | Values are currently unreadable | S |
| 5 | Replace the 33-tile wall with a grouped, searchable list | Maximum space, minimum information today | M |
| 6 | Remove control counts and other engineer vocabulary from user surfaces | Wrong mental model | S |
| 7 | Group the sidebar with separators and section labels | Nine flat items force linear scanning | S |
| 8 | Surface applied-vs-requested divergence per control | Our real advantage, currently invisible | M |
| 9 | Build the **Now** home surface | Answers the question every complaint has been about | L |
| 10 | Consolidate to four surfaces (Now / Sound / Routing / Advanced) | Fixes the subsystem-shaped IA | L |
| 11 | Add the speaker-layout diagram and Test button | Present in Command, absent here | M |
| 12 | Headphone model selection | Present in Command, absent here | M |

Prerequisite for 8–12: finish the module split in `GOAL.md` Phase 4a.
Doing structural UI work in the remaining 4,059-line binary is how
regressions get in.

### Constraints the implementer must not break

- No control may appear that does not map to a real, checked mechanism.
  Honest absence beats a dead widget.
- Creative artwork, icons and branding stay out of the repository. Profile
  cards remain text-based; that is a legal boundary, not a style choice.
- Every write keeps its readback verification. Visual work must not bypass
  the checked mixer path.
- Accessibility floor holds: every interactive control named in the
  accessibility tree, contrast at or above 5.91:1.
- Performance envelope holds: startup < 250 ms, refresh < 100 ms, idle CPU
  ~0%, peak idle RSS ~75 MiB.
