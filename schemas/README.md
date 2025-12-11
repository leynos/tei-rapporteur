# TEI Episodic Profile Schema

This directory contains the ODD (One Document Does it all) specification for the
Text Encoding Initiative (TEI) Episodic Profile, a constrained subset of TEI P5
tailored for podcast scripting and transcription use cases.

## Target TEI Version

This profile is based on **TEI P5 version 4.8.0** (released 2024) and uses the
following TEI modules:

- `tei` – Core infrastructure
- `core` – Core elements (p, hi, title, etc.)
- `header` – TEI header elements
- `textstructure` – Text structure (text, body)
- `spoken` – Spoken transcription (u, pause)
- `namesdates` – Names and dates (listPerson, person)
- `linking` – Linking mechanisms
- `analysis` – Analysis elements (spanGrp, span)
- `transcr` – Transcription elements (listAnnotation)

## Contents

- `tei-episodic-profile.odd` – The ODD customisation defining allowed elements,
  attributes, and validation constraints.

## About the Profile

The TEI Episodic Profile supports:

- **Header metadata**: title, speaker declarations, annotation systems,
  revision history
- **Body structure**: paragraphs (`<p>`) and utterances (`<u>`) with optional
  speaker references
- **Inline elements**: emphasis (`<hi>`), pause markers (`<pause>`)
- **Validation rules**: unique `xml:id` values, speaker cross-referencing

### Speaker Reference Convention

This profile uses **bare identifiers** for speaker references in the `@who`
attribute of utterances. For example:

```xml
<u who="host">Welcome to the show.</u>
```

This differs from TEI's standard `data.pointer` pattern which uses hash
prefixes (e.g., `who="#host"`). The profile does not support:

- Hash-prefixed pointer references (`#host`)
- Multi-valued speaker lists (`host guest`)
- XPointer expressions

Each utterance must reference exactly one speaker by their bare identifier as
declared in `profileDesc/listPerson/person/@xml:id`.

## Generating Schemas

The ODD can be processed by TEI tools to generate Relax NG and Schematron
schemas.

### Using Roma (Web Interface)

1. Visit <https://roma.tei-c.org/>
2. Upload `tei-episodic-profile.odd`
3. Download the generated Relax NG (`.rng`) and Schematron (`.sch`) files

### Using TEI Stylesheets (Command Line)

The [TEI Stylesheets](https://github.com/TEIC/Stylesheets) provide XSLT
transformations for processing ODD files. To install:

```bash
# Clone the TEI Stylesheets repository
git clone https://github.com/TEIC/Stylesheets.git

# Set the environment variable
export TEI_STYLESHEETS=/path/to/Stylesheets
```

Then generate schemas using Saxon or another XSLT processor:

```bash
# Generate Relax NG schema
saxon -xsl:$TEI_STYLESHEETS/odds/odd2relax.xsl \
      -s:tei-episodic-profile.odd \
      -o:tei-episodic-profile.rng

# Generate Schematron rules
saxon -xsl:$TEI_STYLESHEETS/odds/odd2schematron.xsl \
      -s:tei-episodic-profile.odd \
      -o:tei-episodic-profile.sch
```

## Validation

Documents conforming to this profile can be validated against the generated
Relax NG schema using tools such as [jing](https://relaxng.org/jclark/jing.html):

```bash
jing tei-episodic-profile.rng document.xml
```

### Schematron Validation (XPath 2.0 Required)

The Schematron constraints in this profile use XPath 2.0 functions (such as
`exists()`). You must use an **XPath 2.0-capable processor** for Schematron
validation. Saxon is recommended; XPath 1.0-only tools (such as jing's built-in
Schematron support) cannot validate these constraints.

Compile the `.sch` file to XSLT using the ISO Schematron skeleton (available
from [schematron.com](https://schematron.com/front-page/the-schematron-skeleton-implementation/))
and then apply it with Saxon:

```bash
# Compile Schematron to XSLT validator
saxon -xsl:iso_schematron_skeleton_for_saxon.xsl \
      -s:tei-episodic-profile.sch \
      -o:validator.xsl

# Validate a document
saxon -xsl:validator.xsl -s:document.xml
```

## Further Reading

- [TEI Guidelines](https://tei-c.org/release/doc/tei-p5-doc/en/html/)
- [Getting Started with ODD](https://tei-c.org/guidelines/customization/getting-started-with-p5-odds/)
- [Roma](https://roma.tei-c.org/) – Web tool for creating and editing ODDs
- [TEI Stylesheets](https://github.com/TEIC/Stylesheets) – XSLT for ODD
  processing
