"""Tests for extract-majdata.py: python3 -m unittest scripts/test_extract_majdata.py"""

import importlib.util
import os
import struct
import subprocess
import tempfile
import unittest
import zlib

_HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location(
    "extract_majdata", os.path.join(_HERE, "extract-majdata.py")
)
extract = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(extract)

# A slide prefab as Unity writes it, cut down: the root lists its children in
# draw order (m_Children), which is not the order of the documents in the
# file; the last child is the SlideOK judgement, and one child is a nested
# prefab instance whose transform is "stripped".
FIXTURE = """%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &10
GameObject:
  m_Name: Star_Line_5
--- !u!4 &11
Transform:
  m_GameObject: {fileID: 10}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_Children:
  - {fileID: 21}
  - {fileID: 31}
  - {fileID: 41}
  - {fileID: 61}
  - {fileID: 51}
  m_Father: {fileID: 0}
--- !u!1 &40
GameObject:
  m_Name: Slide_0.9
--- !u!4 &41
Transform:
  m_GameObject: {fileID: 40}
  m_LocalRotation: {x: -0, y: -0, z: 0.55557024, w: 0.8314696}
  m_LocalPosition: {x: -1.5, y: -3.5, z: -3.6}
  m_Children: []
  m_Father: {fileID: 11}
--- !u!1 &20
GameObject:
  m_Name: Slide_0.1
--- !u!4 &21
Transform:
  m_GameObject: {fileID: 20}
  m_LocalRotation: {x: -0, y: -0, z: 0.55557024, w: 0.8314696}
  m_LocalPosition: {x: 1.5, y: 3.5, z: -3.6}
  m_Children: []
  m_Father: {fileID: 11}
--- !u!1 &30
GameObject:
  m_Name: Slide_0.5
--- !u!4 &31
Transform:
  m_GameObject: {fileID: 30}
  m_LocalRotation: {x: 0, y: 0, z: 0.55557024, w: 0.8314696}
  m_LocalPosition: {x: 0, y: 0, z: -3.6}
  m_Children: []
  m_Father: {fileID: 11}
--- !u!1 &50
GameObject:
  m_Name: Just_str
--- !u!4 &51
Transform:
  m_GameObject: {fileID: 50}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: -1.6, y: -3.8, z: -4.4}
  m_Children: []
  m_Father: {fileID: 11}
--- !u!4 &61 stripped
Transform:
  m_CorrespondingSourceObject: {fileID: 999}
  m_PrefabInstance: {fileID: 1000}
"""


def png_bytes(width, height):
    """A valid RGBA PNG of the given size, built with the standard library."""

    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    raw = b"".join(b"\x00" + b"\x00" * 4 * width for _ in range(height))
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw))
        + chunk(b"IEND", b"")
    )


class ParsePrefab(unittest.TestCase):
    def test_bars_follow_child_order(self):
        name, bars = extract.parse_prefab(FIXTURE)
        self.assertEqual(name, "Star_Line_5")
        self.assertEqual([b[0] for b in bars], ["Slide_0.1", "Slide_0.5", "Slide_0.9"])
        self.assertEqual(bars[0][1:3], (1.5, 3.5))

    def test_angle_from_quaternion(self):
        _, bars = extract.parse_prefab(FIXTURE)
        for bar in bars:
            self.assertAlmostEqual(bar[3], 67.5, places=3)

    def test_slideok_child_is_dropped(self):
        _, bars = extract.parse_prefab(FIXTURE)
        self.assertNotIn("Just_str", [b[0] for b in bars])


class PngSize(unittest.TestCase):
    def test_png_size(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = os.path.join(tmp, "one.png")
            with open(path, "wb") as f:
                f.write(png_bytes(1, 1))
            self.assertEqual(extract.png_size(path), (1, 1))


class Pin(unittest.TestCase):
    def test_refuses_other_commit(self):
        with tempfile.TemporaryDirectory() as tmp:
            git = ["git", "-C", tmp, "-c", "user.name=t", "-c", "user.email=t@t"]
            subprocess.run(["git", "init", "-q", tmp], check=True)
            subprocess.run(git + ["commit", "-q", "--allow-empty", "-m", "x"], check=True)
            with tempfile.TemporaryDirectory() as out:
                self.assertEqual(extract.main(["--src", tmp, "--out", out]), 2)


if __name__ == "__main__":
    unittest.main()
