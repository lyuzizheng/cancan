# Phone capture Shortcut

`save-to-cancan-inbox.shortcut` is the `Save to CanCan Inbox` Share-sheet
Shortcut: the phone half of the Phase 1 acquisition surface in
[spec 0017](../../docs/specs/0017-evidence-documents-source-ux.md). Sharing a
file from iOS or iPadOS runs it once for each shared file, saves that file into
the CanCan Inbox folder in the user's own iCloud Drive, and shows one receipt.

The artifact is the Shortcut in Apple's pre-signing plist format, so it is
reviewable in Git. `manifest.json` records its version, name, destination,
receipt copy, and digest.

`scripts/shortcuts/build-save-to-cancan-inbox.py` is the only writer of both
files. Regenerate with it; `.agents/scripts/check-shortcut-artifact.sh` (wired
into `agent-preflight.sh`) fails when either drifts from the generator.

## Where the phone writes

The Share sheet's **Save File** action writes inside the Shortcuts container,
so the Shortcut saves to `/Cancan/Inbox/`, which the Files app shows as:

    iCloud Drive ▸ Shortcuts ▸ Cancan ▸ Inbox

CanCan's setup therefore authorizes **`iCloud Drive ▸ Shortcuts ▸ Cancan`** as
the CanCan root, and creates nothing except its own `Inbox` and `Backups` names
inside it. The Mac side never depends on the phone's path beyond that folder:
the host resolves the authorized bookmark, captures with the accepted
native-preflight/two-second stable protocol, and copies verified bytes into the
encrypted Vault without changing the source file.

> Device check pending: that the Share sheet's Save File action resolves
> `/Cancan/Inbox/` inside the Shortcuts container on real hardware. The
> generator encodes the destination the same way the acceptance spike recorded
> it; a real-device run is the only way to close that item. If it lands
> elsewhere, that run's observed path replaces `DESTINATION_PATH` in the
> generator and this file.

## Install

From CanCan (the supported path): run the phone-capture setup action. The host

1. writes the embedded artifact into its data directory,
2. signs it with the local Shortcuts CLI (`shortcuts sign -m anyone`), and
3. hands the signed file to Shortcuts, which asks the user to add it.

Signing is preferred, not required: when it fails the unsigned artifact is
opened instead. An unsigned import may need "Allow Untrusted Shortcuts" in the
Shortcuts settings on that Mac.

Manually, for review or for a machine without CanCan:

```sh
shortcuts sign -m anyone \
  -i resources/shortcuts/save-to-cancan-inbox.shortcut \
  -o /tmp/save-to-cancan-inbox.shortcut
open /tmp/save-to-cancan-inbox.shortcut
```

## Real-device test path

Needs the owner's phone and Mac; nothing in the host-side gates can replace it.

1. In CanCan, authorize the CanCan root (`iCloud Drive ▸ Shortcuts ▸ Cancan`) and
   run the Inbox check. It must report the Inbox folder as reachable.
2. On the phone, share any PDF, CSV, or image to **Save to CanCan Inbox**. The
   shortcut must show `Saved to CanCan Inbox. CanCan will process it on your
   Mac.`
3. The file appears in `iCloud Drive ▸ Shortcuts ▸ Cancan ▸ Inbox` on both
   devices.
4. With the Vault unlocked, CanCan's Inbox scan imports it once: the same
   dedupe, parse, Review, and Tasks path every other capture takes, and the
   source file is left as it was.
5. Re-sharing the same file must not import a second copy.

Failure modes worth recording:

- The Share sheet shows no receipt: the Shortcut was not added, or iOS refused
  to run it.
- The receipt appears but no file reaches the Mac: iCloud Drive is off for
  Shortcuts, or the phone saved into a different container.
- The file reaches the Mac but CanCan never imports it: the authorized root is
  a different folder, or the Vault was locked during the attempt.

## Versioning

Bump `SHORTCUT_VERSION` in the generator, regenerate, and install the new
artifact. The host records an Inbox check per artifact version, so a new version
reports `never_checked` until the check runs again, instead of carrying a pass
that was proven with the old artifact.
