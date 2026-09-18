#!/usr/bin/env python3
"""Generate the versioned `Save to CanCan Inbox` Share-sheet Shortcut artifact.

The artifact is a `.shortcut` file in Apple's pre-signing plist format. It is
reviewable text in Git and is signed on the user's Mac when CanCan offers the
explicit install action (`shortcuts sign --mode anyone`), because unsigned
shortcut files cannot be imported on iOS 15 or later.

Run without arguments to rewrite the committed artifact:

    python3 scripts/shortcuts/build-save-to-cancan-inbox.py

The destination path is relative to the Shortcuts app's own iCloud Drive
container. The built-in Save File action is sandboxed to that container, so the
phone cannot write into an iCloud Drive folder outside it; the matching CanCan
root is therefore `iCloud Drive/Shortcuts/Cancan`. See
`docs/specs/0017-evidence-documents-source-ux.md` and
`resources/shortcuts/README.md`.
"""

import hashlib
import json
import plistlib
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
ARTIFACT_PATH = REPO_ROOT / "resources" / "shortcuts" / "save-to-cancan-inbox.shortcut"
MANIFEST_PATH = REPO_ROOT / "resources" / "shortcuts" / "manifest.json"

SHORTCUT_VERSION = 1
SHORTCUT_NAME = "Save to CanCan Inbox"
DESTINATION_PATH = "/Cancan/Inbox/"
RECEIPT_TEXT = "Saved to CanCan Inbox. CanCan will process it on your Mac."

# Fixed identifiers keep the committed artifact byte-stable, so a regeneration
# check can prove the artifact was not hand-edited.
REPEAT_GROUP_IDENTIFIER = "0F1D5C4E-0E7B-4E1C-9E51-2E9A4B2C7A10"
REPEAT_END_UUID = "9C1B7A24-6C2D-4C51-8E0F-3B5D2A9E4C31"
SAVE_ACTION_UUID = "4A7E1C65-9B30-4C2E-8F1D-6E2B7C5A9D42"
RECEIPT_ACTION_UUID = "D2E5F91B-3A7C-4C08-9B6E-1F4A8C2D7E53"

# Only these content types reach the shortcut's Share-sheet entry. CanCan
# accepts PDF, CSV, and image evidence; everything else stays out of the flow.
INPUT_CONTENT_ITEM_CLASSES = [
    "WFPDFContentItem",
    "WFImageContentItem",
    "WFGenericFileContentItem",
]

# Apple's icon glyph/color identifiers, as used by Shortcuts.app itself.
ICON_GLYPH_NUMBER = 59511
ICON_START_COLOR = 4282601983


def _repeat_item_input() -> dict:
    """The current share-sheet item, for an `WFInput`-shaped parameter."""
    return {
        "Value": {"Type": "Variable", "VariableName": "Repeat Item"},
        "WFSerializationType": "WFTextTokenAttachment",
    }


def _shortcut_input() -> dict:
    """Everything the system Share sheet passed to the shortcut."""
    return {
        "Value": {"Type": "ExtensionInput"},
        "WFSerializationType": "WFTextTokenAttachment",
    }


def _literal_token_string(text: str) -> dict:
    """A literal `WFTextTokenString` value (no variables attached)."""
    return {"Value": {"string": text}, "WFSerializationType": "WFTextTokenString"}


def build_workflow() -> dict:
    """The accepted workflow: save each shared file, then show one receipt."""
    return {
        "WFWorkflowActions": [
            {
                "WFWorkflowActionIdentifier": "is.workflow.actions.repeat.each",
                "WFWorkflowActionParameters": {
                    "GroupingIdentifier": REPEAT_GROUP_IDENTIFIER,
                    "WFControlFlowMode": 0,
                    "WFInput": _shortcut_input(),
                },
            },
            {
                "WFWorkflowActionIdentifier": "is.workflow.actions.documentpicker.save",
                "WFWorkflowActionParameters": {
                    "UUID": SAVE_ACTION_UUID,
                    "WFAskWhereToSave": False,
                    "WFFileDestinationPath": DESTINATION_PATH,
                    "WFInput": _repeat_item_input(),
                    "WFOverwrite": False,
                },
            },
            {
                "WFWorkflowActionIdentifier": "is.workflow.actions.repeat.each",
                "WFWorkflowActionParameters": {
                    "GroupingIdentifier": REPEAT_GROUP_IDENTIFIER,
                    "UUID": REPEAT_END_UUID,
                    "WFControlFlowMode": 2,
                },
            },
            {
                "WFWorkflowActionIdentifier": "is.workflow.actions.showresult",
                "WFWorkflowActionParameters": {
                    "UUID": RECEIPT_ACTION_UUID,
                    "Text": _literal_token_string(RECEIPT_TEXT),
                },
            },
        ],
        "WFWorkflowClientVersion": "1200",
        "WFWorkflowHasOutputFallback": False,
        "WFWorkflowHasShortcutInputVariables": True,
        "WFWorkflowIcon": {
            "WFWorkflowIconGlyphNumber": ICON_GLYPH_NUMBER,
            "WFWorkflowIconStartColor": ICON_START_COLOR,
        },
        "WFWorkflowImportQuestions": [],
        "WFWorkflowInputContentItemClasses": INPUT_CONTENT_ITEM_CLASSES,
        "WFWorkflowMinimumClientVersion": 900,
        "WFWorkflowMinimumClientVersionString": "900",
        "WFWorkflowName": SHORTCUT_NAME,
        "WFWorkflowOutputContentItemClasses": [],
        "WFWorkflowTypes": ["ActionExtension"],
    }


def serialize_workflow(workflow: dict) -> bytes:
    """Deterministic plist bytes: sorted keys, XML, fixed indentation."""
    return plistlib.dumps(
        workflow,
        fmt=plistlib.FMT_XML,
        sort_keys=True,
        skipkeys=False,
    )


def build_manifest(artifact_bytes: bytes) -> dict:
    return {
        "artifact": ARTIFACT_PATH.name,
        "destinationPath": DESTINATION_PATH,
        "name": SHORTCUT_NAME,
        "receiptText": RECEIPT_TEXT,
        "sha256": hashlib.sha256(artifact_bytes).hexdigest(),
        "version": SHORTCUT_VERSION,
    }


def main() -> int:
    artifact_bytes = serialize_workflow(build_workflow())
    manifest = build_manifest(artifact_bytes)

    if "--check" in sys.argv:
        current_artifact = ARTIFACT_PATH.read_bytes() if ARTIFACT_PATH.exists() else b""
        current_manifest = (
            json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
            if MANIFEST_PATH.exists()
            else {}
        )
        problems = []
        if current_artifact != artifact_bytes:
            problems.append(f"{ARTIFACT_PATH.relative_to(REPO_ROOT)} is stale")
        if current_manifest != manifest:
            problems.append(
                f"{MANIFEST_PATH.relative_to(REPO_ROOT)} is stale: "
                f"expected {json.dumps(manifest, sort_keys=True)}, "
                f"found {json.dumps(current_manifest, sort_keys=True)}"
            )
        if problems:
            print("Shortcut artifact check failed:")
            for problem in problems:
                print(f"  {problem}")
            print("Regenerate with scripts/shortcuts/build-save-to-cancan-inbox.py")
            return 1
        print(
            f"Shortcut artifact is current: {SHORTCUT_NAME} version "
            f"{SHORTCUT_VERSION} sha256 {manifest['sha256']}"
        )
        return 0

    ARTIFACT_PATH.write_bytes(artifact_bytes)
    MANIFEST_PATH.write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(
        f"Wrote {ARTIFACT_PATH.relative_to(REPO_ROOT)} "
        f"(version {SHORTCUT_VERSION}, sha256 {manifest['sha256']})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
