# Astrophoto

A web app where amateur astrophotographers upload, tag, and share images of the
night sky. This glossary pins the project's canonical language so journeys,
tests, and docs use one word per concept.

## Language

**Photo**:
An uploaded astrophotograph — the core content unit a photographer publishes.
_Avoid_: image, picture, post.

**Draft**:
A Photo not yet published (no `published_at`); visible only to its owner.
_Avoid_: unpublished, WIP.

**Display master**:
The 4096 px, q=85 JPEG derivative of a Photo that the CDN transforms for
delivery — distinct from the archival original.
_Avoid_: thumbnail, rendition.

**Frame**:
Overloaded in the codebase: the `/account/frames` screen is the owner's library
of Photos/Drafts, but in astrophotography a "frame" is a single sub-exposure
(light/dark/flat/bias). Canonical here: say **Photo** for the published/draft
unit; reserve **frame** for sub-exposures/calibration only.
_Avoid_: "frame" as a synonym for a published Photo.

**Setup**:
A saved bundle of Equipment items — a named gear configuration — not a single
piece of gear.
_Avoid_: rig, kit, gear (for the bundle).

**Equipment item**:
A single catalog gear entry (telescope, camera, filter, mount, …).
_Avoid_: gear, component.

**Target**:
The celestial object a Photo depicts (e.g. M31).
_Avoid_: subject, object.

**Appreciation**:
The app's endorsement of a Photo (this product's "like").
_Avoid_: like, favourite, star.

**Handle**:
A user's unique public `@identifier`, used in profile and permalink URLs.
_Avoid_: username, slug.
