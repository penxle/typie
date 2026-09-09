# Run with: uv run --with fonttools --with brotli python scripts/subset-tooltip-font.py
# Matches the web's Pretendard variable source at weight 500. The subset is renamed because
# Pretendard is a Reserved Font Name; its original copyright and OFL remain in the font and app.
from hashlib import sha256
from io import BytesIO
from pathlib import Path
from urllib.request import urlopen

from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

source = urlopen("https://cdn.typie.net/fonts/Pretendard-Variable.woff2").read()
assert sha256(source).hexdigest() == "9599f12fd42fc0bce1cd50b47a0c022e108d7aa64dd0d1bb0ed44f3282d900b4"
font = instantiateVariableFont(TTFont(BytesIO(source)), {"wght": 500}, inplace=True)
font.flavor = None
options = subset.Options()
options.name_IDs = ["*"]
options.name_languages = ["*"]
options.name_legacy = True
options.layout_features += ["ss05", "cv12", "ss18"]
subsetter = subset.Subsetter(options=options)
# Printable ASCII, arrows, and Apple modifier/editing key symbols.
subsetter.populate(unicodes=list(range(0x20, 0x7F)) + list(range(0x2190, 0x2195)) + [
    0x21A9, 0x21B5, 0x21E4, 0x21E5, 0x21E7, 0x2303, 0x2318, 0x2325, 0x232B, 0x238B, 0x2423,
])
subsetter.subset(font)
for name in font["name"].names:
    value = {
        1: "Typie Tooltip Shortcuts", 2: "Medium", 3: "TypieTooltipShortcuts-Medium-1",
        4: "Typie Tooltip Shortcuts Medium", 6: "TypieTooltipShortcuts-Medium",
        16: "Typie Tooltip Shortcuts", 17: "Medium",
    }.get(name.nameID)
    if value is not None:
        name.string = value.encode(name.getEncoding())
font.save(Path(__file__).resolve().parent.parent / "compose/src/commonMain/composeResources/font/tooltip_shortcuts.ttf")
