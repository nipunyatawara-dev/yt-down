#!/usr/bin/env python3
"""Package an already-built app with a quiet drag-to-Applications layout."""
import argparse
from pathlib import Path
import dmgbuild

parser = argparse.ArgumentParser()
parser.add_argument("app", type=Path)
parser.add_argument("output", type=Path)
args = parser.parse_args()
root = Path(__file__).resolve().parent.parent
app = args.app.resolve()
if not app.is_dir():
    parser.error(f"App bundle does not exist: {app}")
args.output.parent.mkdir(parents=True, exist_ok=True)
dmgbuild.build_dmg(str(args.output.resolve()), "YT Down", settings={
    "format": "UDZO",
    "filesystem": "HFS+",
    "files": [str(app)],
    "symlinks": {"Applications": "/Applications"},
    "icon": str(root / "src-tauri/icons/icon.icns"),
    "background": str(root / "packaging/dmg-background.png"),
    "window_rect": ((160, 160), (640, 360)),
    "icon_locations": {"YT Down.app": (170, 154), "Applications": (470, 154)},
    "icon_size": 96,
    "text_size": 13,
    "show_status_bar": False,
    "show_tab_view": False,
    "show_toolbar": False,
    "show_pathbar": False,
    "show_sidebar": False,
    "hide_extensions": ["YT Down.app"],
    "default_view": "icon-view",
    "include_icon_view_settings": True,
    "include_list_view_settings": False,
})
