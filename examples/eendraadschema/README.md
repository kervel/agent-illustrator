# Belgian Eendraadschema (One-Wire Diagram) Example

Work-in-progress reference drawing: a Belgian AREI-compliant electrical one-wire diagram (*eendraadschema*), built in agent-illustrator.

## Files

| File | Description |
|------|-------------|
| `eendraad-symbols.ail` | Symbol library — ~20 AREI templates: sockets, switches, light points, sensors, inline devices |
| `eendraad-symbols.svg` | Rendered demo layout of the full symbol library |
| `1525-eendraadschema.ail` | One-wire diagram for a real house (1525 Kessel-Lo, ~30 circuits). Currently self-contained with its own simplified symbols |
| `1525-eendraadschema.svg` | Rendered diagram |

## Status

This is a WIP — intentionally placed in a subdirectory so `examples/render-all.sh`
does not auto-pick it up. Re-render manually:

```bash
target/release/agent-illustrator examples/eendraadschema/eendraad-symbols.ail \
    --stylesheet-css stylesheets/kapernikov.css \
    > examples/eendraadschema/eendraad-symbols.svg
```

The eventual goal is for `1525-eendraadschema.ail` to consume `eendraad-symbols.ail`
as an included library (feature pending).

## Symbol design & attribution

Symbol geometry was iterated against two references:

- **Ivan Goethals-Jacobs — [Symbolen (SVG)](https://ivan.goethals-jacobs.be/resources/symbols/pic/Symbolen_all.svg)** — comprehensive set of AREI symbols in SVG. Used as the primary visual reference. Thanks to Ivan for publishing them.
- **FGOV — [Symbolen voor eendraadschema's (PDF, NL)](https://economie.fgov.be/sites/default/files/Files/Energy/Symboles-nl.pdf)** — official symbol list from the Belgian federal economy service.

Geometry is a fresh implementation in AIL (not a direct copy); any errors are mine.
