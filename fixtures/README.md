# Fixtures

## `logs/`

A copy of pm4py's test inputs, from `tests/input_data` in pm4py 2.7.23.8
(commit 24a3bf6, <https://github.com/process-intelligence-solutions/pm4py>).
The directory layout is unchanged, and no file is edited. pm4py is licensed
under AGPL-3.0.

The files are vendored so that CI can read them without a pm4py checkout.

Four files are not from pm4py: `ocel/typed.jsonocel`, `ocel/typed.xmlocel`,
`ocel/typed20.jsonocel` and `ocel/typed20.xmlocel` are small synthetic logs
written for ichnos. They cover typed attributes, object changes, qualifiers,
repeated relations and references to unknown objects in each OCEL layout.

## `golden/`

pm4py's outputs on these logs, one JSON file per case at
`golden/<area>/<case>.json`. `tools/golden/generate.py` writes them; see
`tools/golden/README.md`. Do not edit them by hand.
