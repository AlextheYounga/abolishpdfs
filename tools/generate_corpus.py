#!/usr/bin/env python3
"""Generate small license-free PDFs used by the compatibility corpus."""

from pathlib import Path

from embedded_font_pdf import graphics as font_graphics
from embedded_font_pdf import make_pdf, spaced_text as font_spaced_text, text as font_text


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "tests" / "fixtures" / "generated"
FONT_DIR = ROOT / "tests" / "fixtures" / "fonts"


def pdf(pages):
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        None,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        b"<< /Type /ExtGState /ca 0.5 /CA 0.5 >>",
    ]
    page_ids = []
    annotation_ids = []
    for page in pages:
        content = page["content"].encode("ascii")
        content_id = len(objects) + 1
        objects.append(b"<< /Length %d >>\nstream\n%s\nendstream" % (len(content), content))
        annots = b""
        if page.get("uri"):
            annotation_id = len(objects) + 1
            annotation_ids.append(annotation_id)
            objects.append(None)
            annots = b"%d 0 R" % annotation_id
        annots_part = b" /Annots [" + annots + b"]" if annots else b""
        page_id = len(objects) + 1
        page_ids.append(page_id)
        objects.append(
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 %s %s] /CropBox [0 0 %s %s] /Resources << /Font << /F1 3 0 R >> /ExtGState << /GS1 4 0 R >> >> /Contents %d 0 R%s >>"
            % (
                str(page["width"]).encode(),
                str(page["height"]).encode(),
                str(page.get("crop_width", page["width"])).encode(),
                str(page.get("crop_height", page["height"])).encode(),
                content_id,
                annots_part,
            )
        )
    kids = b" ".join(b"%d 0 R" % page_id for page_id in page_ids)
    objects[1] = b"<< /Type /Pages /Kids [" + kids + b"] /Count %d >>" % len(page_ids)

    # Add URI actions after pages and patch annotations to reference them.
    if any(page.get("uri") for page in pages):
        action_id = len(objects) + 1
        objects.append(b"<< /Type /Action /S /URI /URI (https://example.com/) >>")
        for annotation_id in annotation_ids:
            objects[annotation_id - 1] = (
                b"<< /Type /Annot /Subtype /Link /Rect [72 700 220 730] /Border [0 0 0] /A %d 0 R >>"
                % action_id
            )

    output = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offsets = [0]
    for number, body in enumerate(objects, start=1):
        offsets.append(len(output))
        output.extend(b"%d 0 obj\n" % number)
        output.extend(body)
        output.extend(b"\nendobj\n")
    xref = len(output)
    output.extend(b"xref\n0 %d\n0000000000 65535 f \n" % (len(objects) + 1))
    for offset in offsets[1:]:
        output.extend(b"%010d 00000 n \n" % offset)
    output.extend(
        b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n"
        % (len(objects) + 1, xref)
    )
    return bytes(output)


def text(value, x=72, y=720, size=24, matrix=None):
    matrix = matrix or (1, 0, 0, 1, x, y)
    return "BT /F1 %d Tf %s Tm (%s) Tj ET" % (size, " ".join(map(str, matrix)), value)


def layout_break():
    return "BT /F1 24 Tf 1 0 0 1 72 720 Tm (A) Tj 100 0 Td (B) Tj ET"


def main():
    OUTPUT.mkdir(parents=True, exist_ok=True)
    fixtures = {
        "basic-horizontal.pdf": [{"width": 612, "height": 792, "content": text("Hello, PDFium!")}],
        "transforms.pdf": [{
            "width": 612, "height": 792,
            "content": text("Normal", 72, 720) + "\n" + text("Rotated", matrix=(0, 1, -1, 0, 220, 420)),
        }],
        "spacing.pdf": [{
            "width": 612, "height": 792,
            "content": text("A  B", 72, 720) + "\n" + text("C", 72, 680) + "\n" + text("wide", 72, 640),
        }],
        "mixed-styles.pdf": [{
            "width": 612,
            "height": 792,
            "content": text("Small", 72, 720, size=12) + "\n" + text("Large", 150, 720, size=30),
        }],
        "layout-breaks.pdf": [{"width": 612, "height": 792, "content": layout_break()}],
        "page-boxes.pdf": [
            {"width": 612, "height": 792, "crop_width": 500, "crop_height": 700, "content": text("Crop box page")},
            {"width": 400, "height": 400, "content": text("Second page", 40, 350)},
        ],
        "links.pdf": [{
            "width": 612, "height": 792,
            "content": text("Open example"),
            "uri": True,
        }],
        "clipping-transparency.pdf": [{
            "width": 612, "height": 792,
            "content": "q 1 0 0 1 72 680 cm 100 0 0 40 re W n 0.2 0.5 0.8 rg 0.5 gs " + text("Clipped text", 0, 0) + " Q",
        }],
    }
    for name, pages in fixtures.items():
        (OUTPUT / name).write_bytes(pdf(pages))

    sans = FONT_DIR / "LiberationSans-Regular.ttf"
    serif = FONT_DIR / "LiberationSerif-Regular.ttf"
    embedded_fixtures = {
        "embedded-basic.pdf": make_pdf(
            [(sans, "LiberationSans")],
            [{"width": 612, "height": 792, "content": font_text(1, "Embedded TrueType text", 72, 720)}],
        ),
        "embedded-subset-name.pdf": make_pdf(
            [(sans, "ABCDEF+LiberationSans")],
            [{"width": 612, "height": 792, "content": font_text(1, "Subset-style font name", 72, 720)}],
        ),
        "embedded-multiple-fonts.pdf": make_pdf(
            [(sans, "LiberationSans"), (serif, "LiberationSerif")],
            [{
                "width": 612,
                "height": 792,
                "content": font_text(1, "Sans text", 72, 720) + "\n" + font_text(2, "Serif text", 72, 660),
            }],
        ),
        "embedded-size-transform.pdf": make_pdf(
            [(sans, "LiberationSans")],
            [{
                "width": 612,
                "height": 792,
                "content": font_text(1, "Aa Aa", 72, 720, size=16)
                + "\n"
                + font_text(1, "Aa Aa", 72, 640, size=36)
                + "\n"
                + font_text(1, "Rotate", 0, 0, matrix=(0, 1, -1, 0, 260, 420)),
            }],
        ),
        "embedded-vertical-slice.pdf": make_pdf(
            [(sans, "LiberationSans")],
            [
                {
                    "width": 612,
                    "height": 792,
                    "content": "\n".join([
                        font_text(1, "Embedded font vertical slice", 72, 720, size=24),
                        font_spaced_text(1, "Spacing", 72, 670, size=18),
                        font_text(1, "Rotated", 0, 0, matrix=(0, 1, -1, 0, 300, 500)),
                        font_graphics(),
                    ]),
                    "image": True,
                    "uri": True,
                },
                {
                    "width": 612,
                    "height": 792,
                    "content": "\n".join([
                        font_text(1, "Second page", 72, 720, size=20),
                        font_text(1, "Same font, different size", 72, 660, size=32),
                    ]),
                },
            ],
        ),
    }
    for name, data in embedded_fixtures.items():
        (OUTPUT / name).write_bytes(data)


if __name__ == "__main__":
    main()
