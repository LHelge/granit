"""Generate spike fixture notes inside a Granit cave (path given as argv[1])."""
import sys
from pathlib import Path

root = Path(sys.argv[1])
FM = '---\ntags:\n  - spike\ncreated_at: "2026-11-26T14:00:00.000000Z"\nmodified_at: "2026-11-26T14:00:00.000000Z"\n---\n'


def write(rel: str, body: str, frontmatter: bool = True, raw: bool = False) -> None:
    path = root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    text = (FM if frontmatter else "") + body
    # newline="" keeps CRLF and LF exactly as written.
    with open(path, "w", encoding="utf-8", newline="") as f:
        f.write(text)


write("Spike/Formatting tour.md", """# Formatting tour

A paragraph with **bold**, *italic*, ***both***, ~~strikethrough~~, `inline code`
and a [regular link](https://example.com/path?q=1). An autolink: <https://example.com>.

## Lists

- Bullet one
- Bullet two
  - Nested with dash
    * Nested with star
    + Nested with plus
- Bullet three

1. First
2. Second
   1. Second, first
   2. Second, second
10. Tenth (numbering jumps on purpose)

## Tasks

- [ ] Open task
- [x] Done task
  - [ ] Nested open task
- [X] Done with capital X

## Quote and alerts

> A plain block quote
> over two lines.

> [!NOTE]
> A GitHub-style note alert.

> [!WARNING]
> A warning alert with **bold** inside.

## Table

| Name   | Qty | Note            |
| ------ | --: | :-------------: |
| Apples |   3 | fresh           |
| Pears  |  12 | `inline code`   |
| Plums  |   0 | [[Wiki targets]] |

|Tight|table|
|-|-|
|no|padding|

## Code

```rust
fn main() {
    println!("hello, {}", "cave");
}
```

```python
def total(items):
    return sum(cost for _, cost in items)
```

```
A fence without a language.
```

    An indented code block.

---

## Footnotes and breaks

A sentence with a footnote.[^one] And another.[^long-name]

A line ending in two spaces
continues after a hard break, and one ending in a backslash\\
does too.

[^one]: The first footnote.
[^long-name]: The second footnote, with `code`.

Setext heading
==============

Setext subheading
-----------------

***

Final paragraph.
""")

write("Spike/Wiki targets.md", """# Wiki targets

Plain: [[Test note]]
With label: [[Formatting tour|the tour]]
Non-ASCII slug: [[Räksmörgås och kaffe]]
Slug with spaces: [[Long note for scrolling]]
Heading anchor: [[volvo]] and [[saab-anchor|Saab]]
Broken: [[This note does not exist]]
Broken with label: [[Missing|a missing note]]
In inline code, not a link: `[[Test note]]`

```
In a fence, not a link: [[Test note]]
```

## Volvo {#volvo}

Target of the `volvo` anchor.

## Saab {#saab-anchor}

Target of the `saab-anchor` anchor.

## Plain heading without anchor

Not a link target.
""")

write("Spike/Räksmörgås och kaffe.md", """# Räksmörgås och kaffe

Å, ä och ö: Åke åt en räksmörgås på Öland. Ärligt talat var den för dyr.

Dead-key results: é è ê ë ñ ü õ. Option symbols: © @ £ $ ∞ § | [ ] ≈ ± { }.

Emoji: 🍤 ☕ 👍🏽 🇸🇪

Back to [[Wiki targets]].
""")

write("Spike/Raw HTML and oddities.md", """# Raw HTML and oddities

Inline <b>bold HTML</b>, <kbd>Ctrl</kbd>+<kbd>C</kbd> and a <span style="color:red">styled span</span>.

<details>
<summary>Collapsed section</summary>

Hidden *markdown* inside HTML.

</details>

<!-- an HTML comment that must survive -->

<div align="center">
  <img src="missing.png" width="100">
</div>

Escapes: \\* not emphasis \\*, \\[not a link\\], 2 \\< 3, a literal backtick \\`.

Entities: &amp; &lt; &copy; &#8212; &nbsp;.

Tera-looking text: {{ date | date(format="%Y-%m-%d") }} and {% if x %}y{% endif %}.

Math-looking text: $E = mc^2$ and $$\\int_0^1 x\\,dx$$.

A very long line without any break: """ + "word " * 300 + """

*   Bullet with three spaces after the marker
*   Another


Two blank lines above this paragraph.
""")

# Whitespace fidelity: tabs, trailing spaces, no trailing newline, CRLF, blank edges.
write("Spike/Whitespace tabs and trailing.md",
      "# Tabs and trailing whitespace\n\n\tA line starting with a tab.\n- item\n\t- tab-indented child\n\nTrailing spaces after this line   \nTrailing tab after this line\t\n\n```\n\tcode with\ttabs  \n```\n\n\n\nThree blank lines above.\n")
write("Spike/No trailing newline.md", "# No trailing newline\n\nThe file ends right after this sentence.")
write("Spike/Many trailing newlines.md", "# Many trailing newlines\n\nFour newlines follow.\n\n\n\n")
write("Spike/CRLF line endings.md",
      "# CRLF line endings\r\n\r\nEvery line in this body ends with CRLF.\r\n\r\n- [ ] a task\r\n- [x] done\r\n\r\n| a | b |\r\n| - | - |\r\n| 1 | 2 |\r\n")
write("Spike/Mixed line endings.md", "# Mixed line endings\n\nLF line\nCRLF line\r\nLF line again\n")
write("Spike/Leading blank lines.md", "\n\n# Leading blank lines\n\nTwo blank lines precede the heading.\n")
write("Spike/Empty note.md", "")
write("Spike/Only a heading.md", "# Only a heading")
write("Spike/No frontmatter.md", "# No frontmatter\n\nThis file has no YAML block at all.\n", frontmatter=False)
write("Spike/Empty without frontmatter.md", "", frontmatter=False)

# A 6,000+ line note for typing and scrolling tests.
parts = ["# Long note for scrolling\n"]
for i in range(1, 301):
    parts.append(f"\n## Section {i}\n\nParagraph {i} with **bold**, *italic*, `code` and a link to [[Formatting tour]].\n"
                 + "".join(f"Line {j} of section {i}: the quick brown fox jumps over the lazy dog.\n" for j in range(1, 9))
                 + f"\n- [ ] Task {i}.1\n- [x] Task {i}.2\n\n| Key | Value |\n| --- | ----- |\n| n   | {i}   |\n\n```rust\nlet section = {i};\n```\n")
write("Spike/Long note for scrolling.md", "".join(parts))

# Filler notes so the tree has something to scroll: 12 areas x 5 topics x 6 notes.
areas = ["Archive", "Books", "Cooking", "Garden", "Health", "House", "Ideas", "Journal", "People", "Projects", "Travel", "Work"]
for a, area in enumerate(areas):
    for t in range(1, 6):
        for n in range(1, 7):
            slug = f"{area} {t:02d}-{n:02d}"
            nxt = f"{areas[(a + 1) % len(areas)]} {t:02d}-{n:02d}"
            write(f"Bulk/{area}/Topic {t:02d}/{slug}.md",
                  f"# {slug}\n\nFiller note {n} in topic {t} of {area.lower()}.\n\n- See [[{nxt}]]\n- Back to [[Wiki targets]]\n\n- [ ] A todo in {slug}\n")
(root / "Bulk" / "Empty folder").mkdir(parents=True, exist_ok=True)
