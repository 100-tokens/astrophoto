# Handoff: Astrophoto — Phase 8 (Close the MVP)

This addendum extends the main Astrophoto handoff (`README.md` in this folder) with everything new in **Phase 8**. Read the main README first for brand, tokens, base components, and the full Phase 1–7 screens. Phase 8 reuses 100% of those tokens and components — nothing here changes the system, it only extends it.

> **About the design files** — The Phase 8 prototypes in this bundle are **design references created in HTML/React (Babel inline)** — not production code to copy directly. Recreate them as Svelte components in the SvelteKit project, using the established patterns and tokens documented in the main README.

## Fidelity

**High-fidelity (hifi).** Pixel-perfect mockups with final colors, type, spacing, copy. Every measurement and hex value is final.

## What's new in Phase 8

| # | Feature | Files / artboards |
|---|---|---|
| 01 | Settings v2 — extended (Email & Security depth) | `phase8-settings.jsx` → `ScreenSettingsExtended` |
| — | Settings — tabbed alternative IA | `ScreenSettingsTabbed` |
| — | Settings — with 7-day deletion grace banner | `DeletionGraceWrapper` (in HTML) |
| 02 | Password reset — 3-step public flow | `ScreenResetRequest`, `ScreenResetSent`, `ScreenResetSetNew` |
| — | Password change — in-settings dialog | `ScreenChangePassword` |
| 03 | 2FA setup — QR + 6-digit verify | `Screen2FASetup` |
| — | 2FA backup codes (10 codes) | `Screen2FABackup` |
| 04 | Account deletion confirm dialog | `ScreenDeleteConfirm` |
| 05 | My photos v2 desktop — drafts surfaced | `phase8-screens.jsx` → `ScreenMyPhotosV2` |
| — | My photos mobile (390) | `ScreenMyPhotosMobile` |
| 06 | Replace image — modal (recommended) | `ScreenReplaceModal` |
| — | Replace image — dedicated page alt | `ScreenReplacePage` |
| 07 | Polish 8.5 — eyebrow / FollowButton / untitled / mobile AppreciateButton | `ScreenPolish` |

**Recommendations** (when two variants exist):
- Settings IA: **sectioned scroll with sticky left rail** (matches Phase 1–7).
- Replace flow: **modal** (lightweight, contextual). The dedicated page is documented as a fallback when reprocessing is heavy enough to deserve its own URL — defer if not needed.

## New design tokens

Added to `styles.css` (low-opacity surface tints used for callouts and banners):

| Token | Value | Use |
|---|---|---|
| `--bg-accent-tint` | `rgba(232, 164, 58, 0.06)` | Active nav row, accent panels, current-session highlight |
| `--bg-success-tint` | `rgba(107, 142, 78, 0.08)` | "2FA is on" panel |
| `--bg-warning-tint` | `rgba(201, 137, 32, 0.08)` | Drafts callout, password-reset warnings |
| `--bg-danger-tint` | `rgba(168, 69, 58, 0.10)` | Deletion banner & confirm dialog |

No other token additions. No font additions. No new component classes.

## Screen specs

### 01 · Settings v2 — extended (sectioned)

**Layout** — 1440px wide, `<AppHeader auth/>` 64px → optional deletion-grace strip 44px → page header (eyebrow "PREFERENCES" + h1 "Account *settings*" 48px display) → 240px sticky left nav | 720px max content column, 64px gap.

**Left nav** — vertical mono list, 12px font, 0.12em tracking. Items: PROFILE / EQUIPMENT / NOTIFICATIONS / EMAIL & SECURITY / APPEARANCE / SESSIONS / DELETE ACCOUNT (in `--danger`). Active state: `border-left: 1px solid --accent`, `background: --bg-accent-tint`, color `--accent`. Position `sticky; top: 0; align-self: start`.

Footer-of-nav micro-note (10px mono `--fg-faint`):
> ALL CHANGES AUTOSAVE
> EXCEPT EMAIL · PASSWORD
> · 2FA · DELETION

**Section primitive** (`P8Section`) — h2 26px display italic, optional `--danger` color for destructive sections. Description below, 13px `--fg-muted`, 560px max width. 24px gap before children. Bottom border `1px solid --border-subtle`, 40px bottom margin.

**Row primitive** (`P8Row`) — `grid-template-columns: 160px 1fr; gap: 24px; align-items: start`. Left = 11px mono uppercase label + optional 11px hint below. Right = field.

#### Email & Security — depth example
1. **Sign-in identity** section
   - Email row — input prefilled, "Change…" secondary button to its right (separate verification flow). Below: meta line "● VERIFIED 12 JAN 2026 · LAST USED FROM SAINT-ÉTIENNE-LES-ORGUES" in muted mono.
   - Password row — disabled masked input + "Change…" secondary button (opens `ScreenChangePassword` dialog).
2. **Two-factor authentication** section — full-width status panel: `padding: 20px; background: --bg-success-tint; border: 1px solid --success`. Layout: `grid auto 1fr auto`. Shield icon (28px, stroke `--success`) | title "2FA is on" 17px display italic + meta "AUTHY · ADDED 02 FEB 2026 · 8 OF 10 BACKUP CODES UNUSED" | actions: "View backup codes" ghost · "Disable" danger.
3. **Active sessions** section — list of session rows. Each row: `grid auto 1fr auto auto; gap: 20px; padding: 16px 20px; border: 1px solid --border-subtle`. Current session row gets `background: --bg-accent-tint` + amber 8px dot; non-current gets `--border-strong` dot. Row content: device name (display italic) + "· this device" inline-muted-accent for current + 2 lines of mono meta (browser/OS, location/IP). Right column = "X hours ago" + "Revoke" danger button (only on non-current). Footer button: "Sign out of all other sessions" secondary.
4. **Delete account** section — h2 in `--danger`, panel `padding: 20px; background: --bg-danger-tint; border: 1px solid --danger`. Body copy then "Delete account…" danger button.

#### Settings — tabbed alternative
Same shell minus left rail. Tab strip below page title: `display: flex` with `nav-link` items, the active one underlined per the existing `.nav-link.active::after` pattern. "Delete account" pushed to far right in `--danger`. Use only if tabbed is preferred for routing reasons; sectioned is recommended.

#### Settings — deletion-grace banner state
When the account is mid-grace, show a 44px strip directly under `<AppHeader>`:
- Background `--bg-danger-tint`, bottom border `--danger`, mono 12px content.
- Left: "● ACCOUNT MARKED FOR DELETION" in `--danger` + "Permanent removal in **6 days, 14 hours** · 42 frames will be erased" muted/primary.
- Right: "Cancel deletion" link in `--accent` (underlined).

In the same state, replace the "Delete account" section content with a "Cancel deletion · keep my account" primary + "Download my archive (ZIP)" secondary.

---

### 02 · Password reset (3 steps) + in-settings change

All public reset screens are **720×900** centered single-column layouts (matches `ScreenSignUp`).

**Step 01 — Request link** — Wordmark, eyebrow "RESET PASSWORD · 01 OF 03", h1 "We'll send you a link / to *find your way back*." (40px display, italic on the second line), reassurance copy ("single-use, expires in one hour"), email input, "Send reset link" primary, "← Back to sign in" link.

**Step 02 — Check email** — centered layout, reticle mark 56px in accent at top, accent eyebrow "● RESET PASSWORD · 02 OF 03 · CHECK YOUR EMAIL", h1 "A link is on its way / to *marie.dubois@example.fr*" with the email italicised, reassurance paragraph, and a **plain-text email mockup** in a dashed-border block (520px max width):
- Label "EMAIL PREVIEW · PLAIN TEXT" in muted mono.
- 12px JetBrains Mono body inside `<pre>` with `whiteSpace: pre-wrap`, line-height 1.7.
- Headers: From / To / Subject. Body: "Hello Marie, … Open this link … `https://astrophoto.pics/reset/4f8a-2c1d-9b7e` … expires in one hour. … Clear skies, The Astrophoto archive · 52°31′N · 13°24′E".
- Footer actions: "Resend in 0:42" ghost (with countdown) · "Use a different email" secondary.

**Step 03 — Set new password** — Wordmark, accent eyebrow, h1 "Choose a *new password*.", body with sender email, two password inputs. The first has a **strength meter** below: 4-segment row, each `flex: 1; height: 3px`, segments fill in `--accent` based on strength. Caption beneath in accent mono "STRONG · ESTIMATED 200 YEARS TO CRACK". A warning callout above the submit (`--bg-warning-tint`, `--warning` border, 11px mono): "⚠ ALL OTHER SESSIONS WILL BE SIGNED OUT WHEN YOU SAVE." Primary button "Set new password & sign in".

**In-settings change password dialog (`ScreenChangePassword`)** — 640×720 modal, `--bg-raised` background, `--border-default` border. Accent eyebrow "● CHANGE PASSWORD · IN-SETTINGS", h2 "Change password" (32px display italic). Body: 3 password inputs (current / new w/ strength meter / confirm). The current password row gets a small "I don't remember it →" link in accent below — opens the public reset flow. Footer: "Cancel" ghost + "Save new password" primary.

---

### 03 · 2FA — setup + backup codes

Both modals are **1080×880**, `--bg-raised` background, `--border-default` border, 64px padding.

**Setup (Step 01 of 02)** — Accent eyebrow "● TWO-FACTOR · 01 OF 02 · SCAN OR PASTE", h2 36px "Set up *two-factor authentication*". Body grid `320px 1fr; gap: 48px`:
- **Left**: a real QR code rendered as 22×22 SVG bits inside a 264×264 white panel, 232px QR. Below: "OR PASTE THIS SECRET INTO THE APP" muted mono → readonly mono input "JBSW Y3DP EHPK 3PXP" with "Copy" link in accent on the right.
- **Right**: "STEP 02 — ENTER THE 6-DIGIT CODE" label, 6 single-character mono inputs 56×64px each (8px gap), borders in `--accent`, font-size 24px center-aligned. After verification: success line "● CODE VERIFIED · CONTINUING IN 1S…" in `--success`. Below, a dashed-border explainer: "WHAT IF I LOSE MY PHONE?" → answer about backup codes.
- Footer: "Cancel" ghost · "Continue → backup codes" primary.

The QR is for visual fidelity; in production use a real `qrcode` lib (Svelte: `qrcode-svg`) bound to the TOTP secret server-emits.

**Backup codes (Step 02 of 02)** — Same chrome, eyebrow "● TWO-FACTOR · 02 OF 02 · BACKUP CODES". H2 "Save these *somewhere offline*." Body explains: ten one-time codes, each works once. Then a panel `padding: 24px; border: 1px solid --border-default; background: --bg-base` containing a 2-column grid (`gap: 12px 64px`) of 10 codes, each row: `01` faint left, code right, dashed bottom border. Code format `XXXX-XXXX-XXXX` mono 16px 0.06em. Action row below: "📋 Copy all" / "⤓ Download .txt" / "🖨 Print" secondary buttons; right side: "● THESE WILL NOT BE SHOWN AGAIN" warning mono. Footer: confirmation checkbox "I've saved my backup codes somewhere safe." + "Finish 2FA setup" primary.

**Lost-phone recovery flow** — deferred this round. When added: a public sign-in fallback under "Use a backup code" → 14×3 character mono input → consumes one code, prompts to re-enroll.

---

### 04 · Account deletion confirm dialog

640×780 modal. `--bg-raised` background, **`--danger` border** (departs from the default `--border-default` to mark severity).

Eyebrow `--danger` "● DELETE ACCOUNT · CONFIRMATION", h2 "Are you sure?" 32px display italic in `--danger`. Body explains the 7-day grace, hidden-but-recoverable state, and final erasure.

Below: a `--bg-danger-tint` panel listing what will be erased (frames count, integration hours, appreciations, comments, EXIF, handle releasable after 90 days) and what remains ("[deleted]" attribution on others' threads).

Confirmation gate: input requires the user to type the literal phrase **DELETE MY ACCOUNT** (label is in `--danger`, letter-spacing 0.16em). Footer: "Keep my account" secondary · "Begin 7-day deletion" danger button.

After confirmation: navigate user to the Settings deletion-grace state (banner + cancel/download CTAs). Server marks the account `pending_deletion_at = now + 7 days`. Sign-in is the only path that exposes "cancel deletion" (for security).

---

### 05 · My photos v2 — drafts surfaced

#### Desktop (1440 × 1400)
Replaces Phase 5's `ScreenMyPhotos`. Same shell + stats row, but **drafts count** (currently 3) is rendered in `--accent` instead of `--fg-primary` to draw the eye.

Below the title, a new full-width **drafts callout band** at `padding: 24px 64px; background: --bg-warning-tint; border-bottom: 1px solid --warning`:
- Eyebrow line "● 3 DRAFTS · NOT YET PUBLISHED" in `--warning` + body copy + right-side "SEE ALL DRAFTS →" accent link.
- Below: 3-column grid of draft cards (one per visible draft). Each card `padding: 12px; background: --bg-raised; border: 1px dashed --border-default; gap: 12px; flex`. Layout: 80×80 thumbnail (with a 40% black overlay marking it as draft) + flex column with display-italic 15px target name, mono meta "STEP 02 · VERIFYING DATA · 11 DAYS AGO", and two small buttons: "Continue →" primary-sm + "Discard" ghost-sm.

The main table below is unchanged structurally but adds:
- A 3rd filter chip "Drafts · 3"
- Draft rows in the table get `opacity: 0.78`, the thumbnail gets a `1px dashed --warning` border + 40% black overlay, the status chip is non-accent with `--warning` text/border, and the ♡ count column shows `—`.
- A new **Untitled fallback** row demonstrates the polish item: instead of a target name, it shows the filename in display-italic `--fg-muted` followed by a non-italic muted "· UNTITLED · FROM FILENAME" caption.

#### Mobile (390 × 844)
Compact header (16px vertical padding, mark + wordmark left, avatar right), then the page header and stats reduced to 3 inline numbers (PUB / DRAFTS / total integration). Below, a **drafts banner** (margin 8px 20px, `--bg-warning-tint` panel) with eyebrow + body + full-width "Resume drafts →" secondary button. Filter chips horizontal-scroll. Then list rows: 64×64 thumb left, target/meta/chip stacked right, ⋯ at the far right. Draft rows mirror desktop treatment.

---

### 06 · Replace image

#### Modal — primary (1080 × 780)
Triggered from the photo-detail page action menu ("Replace image…"). `--bg-raised`, `--border-default`.

Header row: accent eyebrow "● REPLACE IMAGE · KEEP CAPTION & METADATA", h2 "Swap a better master" (32px display italic), body copy explaining caption/comments/appreciations/EXIF preservation, and a top-right ✕ close ghost button.

Body grid `1fr auto 1fr; gap: 32px`:
- **Current** column — label "CURRENT · POSTED 14 MAR" mono. 4:3 photo. Below: filename + size in mono meta.
- **Arrow** — center column `width: 32px; height: 32px; border-radius: 50%; border: 1px solid --accent; color: --accent` containing "→".
- **New master** column — label "NEW MASTER" in `--accent`. Drop zone `aspect-ratio: 4/3; border: 1px dashed --accent; background: --bg-accent-tint`. Inside: "Drop the new file" 22px display italic accent, body copy "JPG/PNG/TIFF up to 64 MB", "Or pick a file…" secondary-sm. Below: muted mono note about EXIF re-read.

Below the grid, a `--bg-warning-tint` callout: "⚠ HEADS UP" warning-mono + body explaining the previous file is removed from servers and followers see "REPROCESSED · 14 MAR → 02 MAY 2026" on the photo.

Footer: "Cancel" ghost · "Replace image" primary (disabled at 50% opacity until file picked).

#### Dedicated page — alternative (1440 × 1100)
Same idea on its own URL. AppHeader, then page header (eyebrow + h1 "Reprocess & *swap the master*") + 3-step stepper (UPLOAD NEW / ACCEPT EXIF? / CONFIRM SWAP) styled identically to the upload-flow stepper. Body 2-column: current with mini EXIF table preserving appreciations/comments counts, and new-master drop zone with a side panel about reprocessing semantics. Footer: "← Back to photo" / "Save as draft replacement" / "Continue → check EXIF".

Recommend modal unless reprocessing flows grow more elaborate (multi-step adjustments, side-by-side compare).

---

### 07 · Polish 8.5 — micro-fixes

Single 1440 × 1500 artboard documenting four small changes with before/after pairs. Implementation guidance per item:

#### 1. Context-aware eyebrow on logged-in home
- Public eyebrow stays date-based.
- Logged-in eyebrow becomes `● FROM THE N PHOTOGRAPHERS YOU FOLLOW · M NEW` in `--accent`. The factual context (date, weather, moon) drops down into a sub-line below the greeting headline, 13px `--fg-secondary`.

#### 2. FollowButton on photo detail header
Three states with explicit styling:
- **Not following** — `.btn .btn-primary .btn-sm`, label "Follow".
- **Following · default** — `.btn .btn-secondary .btn-sm` with `border-color: --accent-dim; color: --accent`, label "✓ Following".
- **Following · hover** — same shape, `border-color: --danger; color: --danger`, label "Unfollow?".

Animation: on click-to-follow, button briefly fills (150ms primary), settles into "✓ Following" over 240ms.

The avatar + name remain a navigation link to the profile; the button is a separate action and does not navigate.

#### 3. Untitled photo fallback
When `photo.title` is empty:
- **Never** hide the caption block. Silence is suspicious.
- Render filename in **display italic, `--fg-muted`** at the same size the title would have occupied.
- To the right, an inline `.chip` with `border-style: dashed` reading "UNTITLED".
- Meta line below behaves normally.

Apply everywhere a photo title appears: profile cards, gallery cards, detail page header, my-photos table, draft cards.

#### 4. Mobile AppreciateButton (sticky bar)
The Phase 1–7 mobile detail page had an inert placeholder. New spec:
- Sticky bottom bar: 64px tall, `background: --bg-overlay; backdrop-filter: blur(12px); border-top: 1px solid --border-subtle`.
- Heart-and-count pill: 44px tall, `padding: 0 16px`, `border: 1px solid --border-default; border-radius: 999px`. Heart SVG 18px, count in 13px mono.
- **Active state** (you appreciated): pill background `--bg-accent-tint`, border `--accent`, heart filled in `--accent`, count in `--accent`. Increment animates +1 with a 240ms count-up.
- Comment pill (matches), Save button, Share icon button — all 44px tall.
- **Long-press** the heart → bottom sheet with the most-recent appreciators (avatars + names). Tap to dismiss.

---

## Interactions added in Phase 8

- **Settings autosave** — fields autosave on blur, with a small accent dot + "Saved" mono caption next to the field for 2s. Email, password, 2FA, and deletion are explicit-confirm exceptions (button-driven).
- **Reset email throttle** — "Resend in 0:42" ghost button with a server-truthful countdown. Button enabled at 0:00.
- **2FA verify** — when 6 digits entered, server-verified inline. On success, the inputs go `--success` border for 600ms and the page advances. On fail, the row shakes 240ms and clears.
- **Backup codes** — copy/download/print all available. The "I've saved" checkbox blocks the primary action when off; on save, panel fades to a `--success`-bordered "2FA is on" confirmation.
- **Deletion grace** — banner persistent across every authenticated route until cancelled or expired. Cancellation requires an authed POST.
- **Replace** — drag-and-drop OR click-to-pick. Dropping over the modal body shows a full-modal accent dashed overlay.

## Empty / loading / error states

- **No drafts** — drafts callout band hidden entirely.
- **No sessions other than current** — sessions list shows only the current row, "Sign out of all other sessions" hidden.
- **Reset link expired** — Step 03 replaced by a `--bg-danger-tint` panel: "This link has expired or already been used." + primary "Request a new link" → returns to Step 01.
- **2FA verify fails** — inline error mono `--danger`: "● THAT CODE DIDN'T WORK · CHECK YOUR APP'S CLOCK".
- **Replace upload fails** — drop zone keeps the dashed border, switches accent → `--danger`, error mono caption.

## State management notes (SvelteKit)

- `+page.server.ts` for each settings sub-route handles autosave POSTs (form-actions). Email/password/2FA/deletion live in their own routes (`/settings/email`, `/settings/security`, `/settings/security/2fa`, `/settings/delete`) so they can require fresh password challenges.
- 2FA secret + backup codes generated server-side, signed to a session, never re-shown after the page leaves.
- Backup codes stored as bcrypt hashes; verifying a code marks it used.
- Deletion sets `pending_deletion_at`; a daily worker hard-deletes when `pending_deletion_at < now`. Sign-in middleware shows the grace banner and exposes the cancel action.
- Replace flow uses a presigned upload + server-side EXIF parse, same as upload, then a final POST that swaps the storage key while preserving the row id (so URL is stable).
- Drafts: a Photo with `published_at: null`. Resuming opens the upload flow at the appropriate step. Discarding hard-deletes the row + uploaded file.

## Accessibility additions

- All modal dialogs use `role="dialog"`, focus trap, ESC to dismiss, focus restored to invoker.
- 2FA 6-digit inputs: `inputmode="numeric"`, `autocomplete="one-time-code"`, auto-advance on input, paste-fills-all.
- Backup codes block has a single-press "Copy all" with visible confirmation; codes are `<dl>`-structured with `aria-label="Backup code N of 10"`.
- Deletion confirm button stays disabled until typed phrase matches exactly.
- FollowButton — `aria-pressed`, label changes between states.
- Mobile AppreciateButton — `aria-pressed`, count updates announced via `aria-live="polite"`.
- Sticky mobile bar — adds `padding-bottom: env(safe-area-inset-bottom)`.

## Files in this Phase 8 bundle

- **`Astrophoto Design — Phase 8.html`** — entry point. Open this to see all Phase 8 artboards in the design canvas.
- **`phase8-settings.jsx`** — Settings shell + extended/tabbed variants, password reset (3 steps + change), 2FA setup + backup codes, delete confirm.
- **`phase8-screens.jsx`** — My Photos v2 (desktop + mobile), Replace (modal + page), Polish 8.5.
- **`styles.css`** — full design tokens (now including the four new tint tokens). Reuse from main project.

For Phase 1–7 screens, brand, base components, and the broader system reference, see the main `README.md` in this folder.

---

*If anything is unclear, the source HTML in `Astrophoto Design — Phase 8.html` (with `styles.css`) is authoritative — open it, pan around, pick the artboard.*
