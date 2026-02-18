"""WOFF2 → SFNT 변환. stdin에서 WOFF2를 읽고 stdout에 SFNT를 쓴다."""
import sys
from io import BytesIO

from fontTools.ttLib.woff2 import decompress

data = sys.stdin.buffer.read()
out = BytesIO()
decompress(BytesIO(data), out)
sys.stdout.buffer.write(out.getvalue())
