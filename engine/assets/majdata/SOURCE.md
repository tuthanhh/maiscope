# MajdataPlay art and slide layouts

The slide layouts and the effects come from [MajdataPlay](https://github.com/TeamMajdata/MajdataPlay) at commit
`158c78a1bfd2793d9adba33d476c3b37d64599fc`, licensed under the GPL-3.0 (see `LICENSE` and
`LICENSE-THIRD-PARTY` at the repository root).

- `skin/`: the `Deluxe` skin shipped with MajdataPlay's
  Windows build, copied unchanged. It is not part of MajdataPlay's source
  and carries no licence of its own; it looks like SEGA's maimai DX art.
  See the SEGA notice in `LICENSE-THIRD-PARTY`.
- `effects/`: from `Assets/Sprites/Game/EffectSprites`, copied unchanged
- `slides.txt`: each prefab in `Assets/Prefabs/Game/Slides`, its arrows'
  positions and angles in draw order
- `sprites.txt`: each PNG's pixel size

Do not edit these files. To regenerate them, check out that commit and run
`python3 scripts/extract-majdata.py --src <checkout>` `--skin <MajdataPlay build>/Skins/Deluxe`.
