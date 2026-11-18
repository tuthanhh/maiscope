# MajdataPlay art and slide layouts

Everything in this folder comes from [MajdataPlay](https://github.com/TeamMajdata/MajdataPlay) at commit
`158c78a1bfd2793d9adba33d476c3b37d64599fc`, and is licensed under the GPL-3.0 (see `LICENSE` and
`LICENSE-THIRD-PARTY` at the repository root). The skin is likely derived
from SEGA's maimai art; see the SEGA notice in `LICENSE-THIRD-PARTY`.

- `skin/`: from `Assets/StreamingAssets/Skins/default`, copied unchanged
- `effects/`: from `Assets/Sprites/Game/EffectSprites`, copied unchanged
- `slides.txt`: each prefab in `Assets/Prefabs/Game/Slides`, its arrows'
  positions and angles in draw order
- `sprites.txt`: each PNG's pixel size

Do not edit these files. To regenerate them, check out that commit and run
`python3 scripts/extract-majdata.py --src <checkout>`.
