# Embedded Sound Blaster Command profile data

`sbcommand-3.5.10-default-profiles.json` contains the 33 selectable factory
Sound Effects profiles shipped for the AE-5 by Sound Blaster Command 3.5.10.0.
Each entry is converted to the existing native profile schema for both the
speaker and headphone sections.

The conversion used the same validated Rust importer as user-initiated
migration. It retains only profile names, source identifiers, and representable
AE-5 control values. Creative artwork, file paths, binaries, raw configuration
files, and user-created profile contents are not included.

The read-only source snapshot used for this conversion had SHA-256
`642f48ba3b37d28905d7885da92d4e1e345f9578c7af0ea2669b3154d98b1ee3`.
The generated native catalog has SHA-256
`4614a85aba24cd7327000102b775a8b7386d756475cad03ac62808d24b854e84`.
