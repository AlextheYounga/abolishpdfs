#!/usr/bin/env python3
"""Build deterministic PDFs containing embedded TrueType font programs."""

from pathlib import Path
import zlib


class TrueTypeFont:
    def __init__(self, path: Path, pdf_name: str):
        self.path = path
        self.pdf_name = pdf_name
        self.data = path.read_bytes()
        self.units_per_em = self._u16(self._table(b"head"), 18)
        hhea = self._table(b"hhea")
        self.metric_count = self._u16(hhea, 34)
        maxp = self._table(b"maxp")
        self.glyph_count = self._u16(maxp, 4)
        self.cmap = self._cmap()
        self.metrics = self._metrics()

    def width(self, character: str) -> int:
        glyph = self.cmap.get(ord(character), 0)
        advance = self.metrics[min(glyph, len(self.metrics) - 1)]
        return round(advance * 1000 / self.units_per_em)

    def widths(self) -> list[int]:
        return [self.width(chr(codepoint)) for codepoint in range(32, 127)]

    def _table(self, tag: bytes) -> bytes:
        for offset in range(12, 12 + self._u16(self.data, 4) * 16, 16):
            if self.data[offset : offset + 4] == tag:
                start = self._u32(self.data, offset + 8)
                length = self._u32(self.data, offset + 12)
                return self.data[start : start + length]
        raise ValueError(f"missing TrueType table {tag!r} in {self.path}")

    def _cmap(self) -> dict[int, int]:
        table = self._table(b"cmap")
        records = self._u16(table, 2)
        best = None
        for index in range(records):
            record = 4 + index * 8
            platform = self._u16(table, record)
            encoding = self._u16(table, record + 2)
            offset = self._u32(table, record + 4)
            subtable = table[offset:]
            format_number = self._u16(subtable, 0)
            priority = 3 if platform == 3 and encoding in (1, 10) else 2 if platform == 0 else 0
            if format_number == 4 and priority and (best is None or priority > best[0]):
                best = (priority, subtable)
        if best is None:
            raise ValueError(f"no supported Unicode cmap in {self.path}")
        return self._format_four_cmap(best[1])

    def _format_four_cmap(self, table: bytes) -> dict[int, int]:
        segment_count = self._u16(table, 6) // 2
        end_codes = 14
        start_codes = end_codes + segment_count * 2 + 2
        deltas = start_codes + segment_count * 2
        range_offsets = deltas + segment_count * 2
        result = {}
        for codepoint in range(32, 127):
            for index in range(segment_count):
                end = self._u16(table, end_codes + index * 2)
                start = self._u16(table, start_codes + index * 2)
                if not start <= codepoint <= end:
                    continue
                delta = int.from_bytes(table[deltas + index * 2 : deltas + index * 2 + 2], "big", signed=True)
                range_offset = self._u16(table, range_offsets + index * 2)
                if range_offset == 0:
                    glyph = (codepoint + delta) & 0xFFFF
                else:
                    glyph_offset = range_offsets + index * 2 + range_offset + (codepoint - start) * 2
                    glyph = self._u16(table, glyph_offset)
                    if glyph:
                        glyph = (glyph + delta) & 0xFFFF
                result[codepoint] = glyph
                break
        return result

    def _metrics(self) -> list[int]:
        table = self._table(b"hmtx")
        values = [self._u16(table, index * 4) for index in range(self.metric_count)]
        values.extend([values[-1]] * (self.glyph_count - len(values)))
        return values

    @staticmethod
    def _u16(data: bytes, offset: int) -> int:
        return int.from_bytes(data[offset : offset + 2], "big")

    @staticmethod
    def _u32(data: bytes, offset: int) -> int:
        return int.from_bytes(data[offset : offset + 4], "big")


class PdfBuilder:
    def __init__(self):
        self.objects: list[bytes | None] = [None, None]

    def add(self, body: bytes) -> int:
        self.objects.append(body)
        return len(self.objects)

    def stream(self, dictionary: bytes, data: bytes) -> int:
        return self.add(dictionary + b" /Length " + str(len(data)).encode() + b" >>\nstream\n" + data + b"\nendstream")

    def finish(self, pages: list[int]) -> bytes:
        self.objects[0] = b"<< /Type /Catalog /Pages 2 0 R >>"
        kids = b" ".join(f"{page} 0 R".encode() for page in pages)
        self.objects[1] = b"<< /Type /Pages /Kids [" + kids + b"] /Count " + str(len(pages)).encode() + b" >>"
        output = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
        offsets = [0]
        for number, body in enumerate(self.objects, start=1):
            if body is None:
                raise ValueError(f"unassigned PDF object {number}")
            offsets.append(len(output))
            output.extend(f"{number} 0 obj\n".encode())
            output.extend(body)
            output.extend(b"\nendobj\n")
        xref = len(output)
        output.extend(f"xref\n0 {len(self.objects) + 1}\n0000000000 65535 f \n".encode())
        for offset in offsets[1:]:
            output.extend(f"{offset:010d} 00000 n \n".encode())
        output.extend(f"trailer\n<< /Size {len(self.objects) + 1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode())
        return bytes(output)


def embedded_font(builder: PdfBuilder, font: TrueTypeFont) -> int:
    stream = builder.stream(b"<< /Length1 " + str(len(font.data)).encode(), font.data)
    descriptor = builder.add(
        f"<< /Type /FontDescriptor /FontName /{font.pdf_name} /Flags 32 /FontBBox [-1000 -300 2000 1100] "
        f"/ItalicAngle 0 /Ascent 900 /Descent -220 /CapHeight 700 /StemV 80 /FontFile2 {stream} 0 R >>".encode()
    )
    widths = " ".join(str(width) for width in font.widths())
    return builder.add(
        f"<< /Type /Font /Subtype /TrueType /BaseFont /{font.pdf_name} /FirstChar 32 /LastChar 126 "
        f"/Widths [{widths}] /FontDescriptor {descriptor} 0 R /Encoding /WinAnsiEncoding >>".encode()
    )


def image(builder: PdfBuilder) -> int:
    pixels = bytes((236, 80, 74, 58, 126, 184, 46, 190, 116, 245, 205, 66))
    compressed = zlib.compress(pixels, level=9)
    return builder.stream(
        b"<< /Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode",
        compressed,
    )


def uri_annotation(builder: PdfBuilder) -> tuple[int, int]:
    action = builder.add(b"<< /Type /Action /S /URI /URI (https://example.com/embedded-font-fixture) >>")
    annotation = builder.add(f"<< /Type /Annot /Subtype /Link /Rect [72 700 240 730] /Border [0 0 0] /A {action} 0 R >>".encode())
    return action, annotation


def make_pdf(font_paths: list[tuple[Path, str]], pages: list[dict]) -> bytes:
    builder = PdfBuilder()
    fonts = [TrueTypeFont(path, pdf_name) for path, pdf_name in font_paths]
    font_ids = [embedded_font(builder, font) for font in fonts]
    image_id = image(builder) if any(page.get("image") for page in pages) else None
    annotations = []
    for page in pages:
        if page.get("uri"):
            annotations.append(uri_annotation(builder)[1])
        else:
            annotations.append(None)

    page_ids = []
    for index, page in enumerate(pages):
        content = page["content"].encode("ascii")
        content_id = builder.stream(b"<<", content)
        font_resources = b" ".join(f"/F{font_index + 1} {font_id} 0 R".encode() for font_index, font_id in enumerate(font_ids))
        resources = b"/Font << " + font_resources + b" >>"
        if image_id is not None and page.get("image"):
            resources += f" /XObject << /Im1 {image_id} 0 R >>".encode()
        annots = b""
        if annotations[index] is not None:
            annots = f" /Annots [{annotations[index]} 0 R]".encode()
        page_ids.append(
            builder.add(
                f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {page['width']} {page['height']}] "
                f"/CropBox [0 0 {page.get('crop_width', page['width'])} {page.get('crop_height', page['height'])}] "
                f"/Resources << ".encode()
                + resources
                + f" >> /Contents {content_id} 0 R{annots} >>".encode()
            )
        )
    return builder.finish(page_ids)


def text(font: int, value: str, x: int, y: int, size: int = 24, matrix: tuple[int, ...] | None = None) -> str:
    transform = matrix or (1, 0, 0, 1, x, y)
    escaped = value.replace("\\", "\\\\").replace("(", "\\(").replace(")", "\\)")
    return f"BT /F{font} {size} Tf {' '.join(map(str, transform))} Tm ({escaped}) Tj ET"


def spaced_text(font: int, value: str, x: int, y: int, size: int = 24) -> str:
    escaped = value.replace("(", "\\(").replace(")", "\\)")
    return f"BT /F{font} {size} Tf 1 0 0 1 {x} {y} Tm [({escaped[:1]}) 120 ({escaped[1:]})] TJ ET"


def graphics(image_name: str = "/Im1") -> str:
    return f"q 90 0 0 70 72 540 cm {image_name} Do Q\n0.2 0.5 0.8 rg 72 460 190 50 re f"
