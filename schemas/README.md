# TEI Episodic Profile Schema

This directory contains the ODD (One Document Does it all) specification for the
TEI Episodic Profile, a constrained subset of TEI P5 tailored for podcast
scripting and transcription use cases.

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

## Generating Schemas

The ODD can be processed by TEI tools to generate Relax NG and Schematron
schemas.

### Using Roma (Web Interface)

1. Visit <https://roma.tei-c.org/>
2. Upload `tei-episodic-profile.odd`
3. Download the generated Relax NG (`.rng`) and Schematron (`.sch`) files

### Using TEI Stylesheets (Command Line)

If the TEI Stylesheets are installed locally:

```bash
# Generate Relax NG schema
saxon -xsl:$TEISTY/odds/odd2relax.xsl \
      -s:tei-episodic-profile.odd \
      -o:tei-episodic-profile.rng

# Generate Schematron rules
saxon -xsl:$TEISTY/odds/odd2schematron.xsl \
      -s:tei-episodic-profile.odd \
      -o:tei-episodic-profile.sch
```

Where `$TEISTY` points to the TEI Stylesheets installation directory.

## Validation

Documents conforming to this profile can be validated against the generated
Relax NG schema using tools such as `jing`:

```bash
jing tei-episodic-profile.rng document.xml
```

For Schematron validation, first compile the `.sch` to XSLT and then apply it:

```bash
saxon -xsl:iso_schematron_skeleton.xsl -s:tei-episodic-profile.sch -o:validator.xsl
saxon -xsl:validator.xsl -s:document.xml
```

## Further Reading

- [TEI Guidelines](https://tei-c.org/release/doc/tei-p5-doc/en/html/)
- [Getting Started with ODD](https://tei-c.org/guidelines/customization/getting-started-with-p5-odds/)
- [Roma](https://roma.tei-c.org/) – Web tool for creating and editing ODDs
