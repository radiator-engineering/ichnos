"""Object-centric event logs: pm4py's ``OCEL`` object.

Each case reads one OCEL fixture with pm4py and records its tables, then
what ``OCEL`` computes from them: ``is_ocel20``, ``get_summary`` and
``get_extended_table``.

The tables are typed rows. An attribute is a ``[type, value]`` pair, with
type ``string``, ``int``, ``float``, ``boolean`` or ``date``, and dates in
UTC ISO format. pandas' missing values (``NaN``, ``None``) are left out. A
relation's qualifier is ``null`` when pandas holds a missing value there.
The relations table repeats each event's activity and timestamp and each
object's type. The golden keeps only the ids and the qualifier, and the
test checks that the repeated columns agree with the events and objects.

pm4py's CSV reader orders objects, and its OCEL 2.0 JSON reader orders each
event's relations, by iterating Python sets of strings. So every read runs
in its own process with ``PYTHONHASHSEED=0``.
"""

import json
import numbers
import os
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

from harness import case  # imports pm4py quietly; keep it first

import pandas as pd
import pm4py
from pm4py.objects.ocel import constants


def _missing(value):
    if value is None:
        return True
    try:
        return bool(pd.isna(value))
    except (TypeError, ValueError):
        return False


def _date(value):
    if _missing(value):
        return None
    value = pd.Timestamp(value)
    if value.tzinfo is None:
        value = value.tz_localize("UTC")
    return value.astimezone(timezone.utc).isoformat(timespec="microseconds")


def _value(value):
    if isinstance(value, bool):
        return ["boolean", value]
    if isinstance(value, (pd.Timestamp, datetime)):
        return ["date", _date(value)]
    if isinstance(value, numbers.Integral):
        return ["int", int(value)]
    if isinstance(value, numbers.Real):
        return ["float", float(value)]
    return ["string", str(value)]


def _attributes(row, skip):
    return {str(k): _value(v) for k, v in row.items() if k not in skip and not _missing(v)}


def _qualifier(value):
    return None if _missing(value) else str(value)


def _tables(ocel):
    eid, oid, otype = ocel.event_id_column, ocel.object_id_column, ocel.object_type_column
    act, ts, qual, field = ocel.event_activity, ocel.event_timestamp, ocel.qualifier, ocel.changed_field
    events = [{"id": str(r[eid]), "activity": str(r[act]), "timestamp": _date(r[ts]),
               "attributes": _attributes(r, {eid, act, ts})}
              for r in ocel.events.to_dict("records")]
    # A column the objects table lacks is null.
    objects = [{"id": str(r[oid]) if oid in r else None,
                "type": str(r[otype]) if otype in r else None,
                "attributes": _attributes(r, {oid, otype})}
               for r in ocel.objects.to_dict("records")]
    relations = [{"event": str(r[eid]), "object": str(r[oid]), "qualifier": _qualifier(r.get(qual)),
                  "activity": str(r[act]), "timestamp": _date(r[ts]), "type": str(r[otype])}
                 for r in ocel.relations.to_dict("records")]
    o2o = [{"source": str(r[oid]), "target": str(r[oid + "_2"]), "qualifier": _qualifier(r.get(qual))}
           for r in ocel.o2o.to_dict("records")]
    e2e = [{"source": str(r[eid]), "target": str(r[eid + "_2"]), "qualifier": _qualifier(r.get(qual))}
           for r in ocel.e2e.to_dict("records")]
    changes = []
    for r in ocel.object_changes.to_dict("records"):
        name = str(r[field])
        value = r.get(name)
        changes.append({"object": str(r[oid]), "type": str(r[otype]), "timestamp": _date(r[ts]),
                        "field": name, "value": None if _missing(value) else _value(value)})
    return {"events": events, "objects": objects, "relations": relations,
            "o2o": o2o, "e2e": e2e, "object_changes": changes}


def _summary(ocel):
    act, otype = ocel.event_activity, ocel.object_type_column
    return {
        "events": len(ocel.events),
        "objects": len(ocel.objects),
        "activities": int(ocel.events[act].nunique()),
        "object_types": int(ocel.objects[otype].nunique()),
        "relations": len(ocel.relations),
        "activity_counts": {str(k): int(v) for k, v in ocel.events[act].value_counts().items()},
        "object_type_counts": {str(k): int(v) for k, v in ocel.objects[otype].value_counts().items()},
        "activities_per_object_type": {
            str(k): int(v) for k, v in ocel.relations.groupby(otype)[act].nunique().items()},
        "text": ocel.get_summary(),
    }


def _extended(ocel):
    prefix = constants.DEFAULT_OBJECT_TYPE_PREFIX_EXTENDED
    table = ocel.get_extended_table()
    types = [c[len(prefix):] for c in table.columns if c.startswith(prefix)]
    rows = []
    for r in table.to_dict("records"):
        objects = {t: [str(o) for o in r[prefix + t]] for t in types
                   if isinstance(r[prefix + t], list)}
        rows.append({"event": str(r[ocel.event_id_column]), "objects": objects})
    return {"object_types": types, "rows": rows}


def summarize(ocel):
    return {
        "ocel": _tables(ocel),
        "is_ocel20": bool(ocel.is_ocel20()),
        "summary": _summary(ocel),
        "extended_table": _extended(ocel),
    }


_READERS = {
    "example_log.jsonocel": ("pm4py.read_ocel_json", pm4py.read_ocel_json),
    "example_log.xmlocel": ("pm4py.read_ocel_xml", pm4py.read_ocel_xml),
    "example_log.csv": ("pm4py.read_ocel_csv", pm4py.read_ocel_csv),
    "newocel.jsonocel": ("pm4py.read_ocel_json", pm4py.read_ocel_json),
    "ocel20_example.jsonocel": ("pm4py.read_ocel2_json", pm4py.read_ocel2_json),
    "ocel20_example.xmlocel": ("pm4py.read_ocel2_xml", pm4py.read_ocel2_xml),
    # Synthetic logs with typed attributes, object changes, qualifiers,
    # repeated relations and references to unknown objects.
    "typed.jsonocel": ("pm4py.read_ocel_json", pm4py.read_ocel_json),
    "typed.xmlocel": ("pm4py.read_ocel_xml", pm4py.read_ocel_xml),
    "typed20.jsonocel": ("pm4py.read_ocel2_json", pm4py.read_ocel2_json),
    "typed20.xmlocel": ("pm4py.read_ocel2_xml", pm4py.read_ocel2_xml),
    # Synthetic CSV logs: Python list literals, NA tokens, ragged rows and
    # quoting in OCEL 1.0; references, JSON attributes, object rows, o2o
    # rows and type inference in OCEL 2.0.
    "typed.csv": ("pm4py.read_ocel_csv", pm4py.read_ocel_csv),
    "typed20.ocel.csv": ("pm4py.read_ocel2_csv", pm4py.read_ocel2_csv),
}


def _read_pinned(rel, path):
    """Reads ``path`` with the reader for ``rel`` under ``PYTHONHASHSEED=0``."""
    golden_tools = str(Path(__file__).resolve().parent.parent)
    env = dict(os.environ, PYTHONHASHSEED="0")
    env["PYTHONPATH"] = os.pathsep.join(filter(None, [golden_tools, env.get("PYTHONPATH")]))
    out = subprocess.run([sys.executable, __file__, rel, str(path)], env=env,
                         check=True, capture_output=True, text=True)
    return json.loads(out.stdout)


for _rel, (_name, _read) in _READERS.items():
    def _run(fixtures, _rel=_rel):
        return _read_pinned(_rel, fixtures["log"])

    case("model-" + _rel.replace(".", "-").replace("_", "-"), fixture="ocel/" + _rel,
         functions=["pm4py.OCEL", _name])(_run)


@case("read-typed-csv-objects", fixtures={"log": "ocel/typed.csv", "objects": "ocel/typed-objects.csv"},
      functions=["pm4py.read_ocel_csv"])
def _typed_csv_objects(fixtures):
    # With an objects file, the objects come from it in file order, and
    # relations may name objects it lacks.
    return {"ocel": _tables(pm4py.read_ocel_csv(str(fixtures["log"]), str(fixtures["objects"])))}


@case("model-empty", functions=["pm4py.OCEL"])
def _empty(fixtures):
    return summarize(pm4py.OCEL())


# Strings for the readers' timestamp parser. Each records the UTC time
# pm4py gives, or null when it raises.
_TIMESTAMPS = [
    "2022-01-09T14:00:00", "2022-01-09T14:00:00Z", "2022-01-09T14:00:00z",
    "2022-01-09T14:00:00.000Z", "2022-01-09 14:00:00", "2022-01-09T15:00:00+01:00",
    "2022-01-09T14:00", "2022-01-09", "2022-01-09T14", "20220109T140000",
    "20220109", "2022-01-09T14:00:00,5", "2022-01-09T15:00:00+01",
    "2022-01-09T15:00:00+0100", "2022-01-09T13:00:00-01:00",
    "2022-01-09T14:00:00+01:00:30", "2022-01-09T14:00:00+01:00:30.5",
    "2022-01-09T14:00:00+01:75", "2022-W01-1", "2022W011", "2022W01", "2022-W01",
    "2022-W01-1T10:00", "2022-W01T10:00", "2022W01T10:00", "2022W0110:00",
    "2022-01-09T14:00:00.1234567Z", "2022-01-09T14:00:00.123",
    "2022-01-09T14:00:00.1234565", "2022-01-09T14:00:00.123456789+01:00",
    " 2022-01-09T14:00:00", "2022-01-09T14:00:00 ", "2022-01-09x14:00:00",
    "2022-01-09T1400", "2022-01-09T140000.5", "2022-01-09T14:00:00:5",
    "2022-01-09T140000123", "2022-01-09T24:00:00", "2022-01-09T14:00:60",
    "2022-02-30", "2024-02-29", "0000-01-01", "0001-01-01", "9999-12-31T23:59:59",
    "2022-01-09T14:00:00+24:00", "2022-01-09T14:00:00+23:59", "2022-01-09T14:00:",
    "2022-01-09T14:00:00.", "2022-01-09T14:00:00.5x", "2022-01-09Z",
    "2022-01-09T14:00:00Z+01:00", "2022-01-09T14:00:00ZZ", "2022-01-09T14Z",
    "2022-01-09T14:00:00-00:00", "2022-01-09T14:00:00+01:00Z", "2020-W53-5",
    "2021-W53-1", "2022-W00-1", "2022-W01-8", "2022-001", "2022-1-9",
    "2022-01-09T", "yesterday", "", "1970-01-01T00:00:00", "0",
    "2022-01-09T14:00:00.5Z", "2022-01-09\u00e914:00", "2022-01-09T\u0661\u0664:00",
]


@case("timestamps", functions=["pm4py.read_ocel_json", "pm4py.read_ocel2_json"])
def _timestamps(fixtures):
    from pm4py.util.dt_parsing import parser

    parse = parser.get().apply
    out = []
    for text in _TIMESTAMPS:
        try:
            value = parse(text).astimezone(timezone.utc).isoformat()
        except Exception:
            value = None
        out.append([text, value])
    return {"timestamps": out}


def _read_text(read, text, suffix):
    """Reads ``text`` from a temporary file named ``*suffix``. Records the
    tables, with the objects sorted by id and type because pm4py's OCEL 1.0
    CSV reader orders them by set iteration, or pm4py's error type."""
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / f"log{suffix}"
        path.write_bytes(text.encode("utf-8"))
        try:
            tables = _tables(read(str(path)))
        except Exception as e:
            return {"error": type(e).__name__}
    tables["objects"].sort(key=lambda o: (o["id"], o["type"]))
    return {"ocel": tables}


_H1 = "ocel:eid,ocel:activity,ocel:timestamp,ocel:type:a,ocel:type:b,x\n"

# OCEL 1.0 CSV texts: pandas' timestamp format guess, NA tokens, quoting,
# ragged rows and Python list literals.
_CSV_TEXTS = {
    "ts-seconds": _H1 + "e1,a,2022-01-09 14:00:00,['o1'],,\ne2,a,2022-01-08 10:00:00,['o1'],,\n",
    "ts-date": _H1 + "e1,a,2022-01-09,['o1'],,\ne2,a,2022-01-08,['o1'],,\n",
    "ts-minutes": _H1 + "e1,a,2022-01-09 14:00,['o1'],,\ne2,a,2022-01-09 15:30,['o1'],,\n",
    "ts-hours": _H1 + "e1,a,2022-01-09 14,['o1'],,\ne2,a,2022-01-09 15,['o1'],,\n",
    "ts-t": _H1 + "e1,a,2022-01-09T14:00:00,['o1'],,\ne2,a,2022-01-09T15:00:00,['o1'],,\n",
    "ts-t-then-space": _H1 + "e1,a,2022-01-09T14:00:00,['o1'],,\ne2,a,2022-01-09 15:00:00,['o1'],,\n",
    "ts-fraction": _H1 + "e1,a,2022-01-09 14:00:00.5,['o1'],,\ne2,a,2022-01-09 15:00:00.123456,['o1'],,\n",
    "ts-fraction-then-none": _H1 + "e1,a,2022-01-09 14:00:00.5,['o1'],,\ne2,a,2022-01-09 15:00:00,['o1'],,\n",
    "ts-none-then-fraction": _H1 + "e1,a,2022-01-09 14:00:00,['o1'],,\ne2,a,2022-01-09 15:00:00.5,['o1'],,\n",
    "ts-nanoseconds": _H1 + "e1,a,2022-01-09 14:00:00.123456789,['o1'],,\n",
    "ts-z": _H1 + "e1,a,2022-01-09T14:00:00Z,['o1'],,\ne2,a,2022-01-09T15:00:00.000Z,['o1'],,\n",
    "ts-z-fraction": _H1 + "e1,a,2022-01-09T14:00:00.000Z,['o1'],,\ne2,a,2022-01-09T15:00:00.250Z,['o1'],,\n",
    "ts-offset": _H1 + "e1,a,2022-01-09T14:00:00+01:00,['o1'],,\ne2,a,2022-01-09T15:00:00+01:00,['o1'],,\n",
    "ts-offset-compact": _H1 + "e1,a,2022-01-09T14:00:00+0100,['o1'],,\n",
    "ts-mixed-offsets": _H1 + "e1,a,2022-01-09T14:00:00+01:00,['o1'],,\ne2,a,2022-01-09T15:00:00+02:00,['o1'],,\n",
    "ts-z-then-offset": _H1 + "e1,a,2022-01-09T14:00:00Z,['o1'],,\ne2,a,2022-01-09T15:00:00+00:00,['o1'],,\n",
    "ts-naive-then-z": _H1 + "e1,a,2022-01-09T14:00:00,['o1'],,\ne2,a,2022-01-09T15:00:00Z,['o1'],,\n",
    "ts-missing": _H1 + "e1,a,,['o1'],,\ne2,a,2022-01-09 15:00:00,['o1'],,\n",
    "ts-day-first": _H1 + "e1,a,20/05/2019 07:07,['o1'],,\n",
    "ts-garbage": _H1 + "e1,a,yesterday,['o1'],,\n",
    "ts-invalid-date": _H1 + "e1,a,2022-02-30 00:00:00,['o1'],,\n",
    "na-attributes": _H1 + "e1,a,2022-01-09 14:00:00,['o1'],,null\ne2,b,2022-01-09 15:00:00,['o1'],,#N/A\n"
                           "e3,c,2022-01-09 16:00:00,['o1'],,None\ne4,d,2022-01-09 17:00:00,['o1'],, NA\n"
                           "e5,e,2022-01-09 18:00:00,['o1'],,<NA>\ne6,f,2022-01-09 19:00:00,['o1'],,nan\n",
    "na-activity": _H1 + "e1,NA,2022-01-09 14:00:00,['o1'],,\n",
    "na-activity-unrelated": _H1 + "e1,n/a,2022-01-09 14:00:00,,,\ne2,b,2022-01-09 15:00:00,['o1'],,\n",
    "lists": _H1 + "e1,a,2022-01-09 14:00:00,\"[ 'a' , \"\"b\"\" ]\",\"['a\\tb', '\\x41\\u00e9', 'q\\\\'s']\",\n"
                   "e2,a,2022-01-09 15:00:00,\"['a', 'a']\",['a'],\n"
                   "e3,a,2022-01-09 16:00:00,\"[u'c' \"\"d\"\"]\",\"[r'\\n']\",\n"
                   "e4,a,2022-01-09 17:00:00,\"('t',)\",\"[]\",\n"
                   "e5,a,2022-01-09 18:00:00,\"['x',]\",\"['y'] \",\n"
                   "e6,a,2022-01-09 19:00:00,\"['''t''']\",\"['\\\\\\n']\",\n",
    "list-named-escape": _H1 + "e1,a,2022-01-09 14:00:00,\"['\\N{BULLET}']\",,\n",
    "list-numbers": _H1 + "e1,a,2022-01-09 14:00:00,\"[1, 2.5]\",,\n",
    "list-bool": _H1 + "e1,a,2022-01-09 14:00:00,[True],,\n",
    "list-none": _H1 + "e1,a,2022-01-09 14:00:00,\"['a', None]\",,\n",
    "list-nested": _H1 + "e1,a,2022-01-09 14:00:00,\"[['a']]\",,\n",
    "list-dict": _H1 + "e1,a,2022-01-09 14:00:00,\"[{'a': 1}]\",,\n",
    "list-bytes": _H1 + "e1,a,2022-01-09 14:00:00,\"[b'x']\",,\n",
    "list-bad": _H1 + "e1,a,2022-01-09 14:00:00,\"['a' 1]\",[,\n",
    "no-type-columns": "ocel:eid,ocel:activity,ocel:timestamp,x\ne1,a,2022-01-09 14:00:00,1\n",
    "header-only": _H1,
    "empty": "",
    "bom": "﻿" + _H1 + "e1,a,2022-01-09 14:00:00,['o1'],,\n",
    "crlf": _H1.replace("\n", "\r\n") + "e1,a,2022-01-09 14:00:00,['o1'],,\"x\r\ny\"\r\n",
    "blank-lines": _H1 + "\ne1,a,2022-01-09 14:00:00,['o1'],,\n\n\n",
    "quotes": _H1 + "e1,a,2022-01-09 14:00:00,['o1'],,a\"b\ne2,a,2022-01-09 15:00:00,['o1'],,\"a\"\"b\"\"\"\n",
    "short-row": _H1 + "e1,a,2022-01-09 14:00:00,['o1']\ne2,a,2022-01-09 15:00:00,['o1'],,1\n",
    "long-row-first": _H1 + "e1,a,2022-01-09 14:00:00,['o1'],,1,2\n",
    "long-row-first-empty": _H1 + "e1,a,2022-01-09 14:00:00,['o1'],,1,\n",
    "long-row-later": _H1 + "e1,a,2022-01-09 14:00:00,['o1'],,1\ne2,a,2022-01-09 15:00:00,['o1'],,1,2\n",
    "long-row-later-empty": _H1 + "e1,a,2022-01-09 14:00:00,['o1'],,1\ne2,a,2022-01-09 15:00:00,['o1'],,1,\n",
    "long-rows-empty": _H1 + "e1,a,2022-01-09 14:00:00,['o1'],,1,\ne2,a,2022-01-09 15:00:00,['o1'],,1,\n",
    "long-row-first-two": _H1 + "e1,a,2022-01-09 14:00:00,['o1'],,1,2,3\ne2,a,2022-01-09 15:00:00,['o1'],,1,2\n",
    "space-line": _H1 + " \t \ne1,a,2022-01-09 14:00:00,['o1'],,\n",
    "lone-cr": _H1.replace("\n", "\r") + "e1,a,2022-01-09 14:00:00,['o1'],,x\re2,a,2022-01-09 15:00:00,['o1'],,\r",
    "quote-then-text": _H1 + "e1,a,2022-01-09 14:00:00,['o1'],,\"a\"x\n",
    "quoted-na": _H1 + "e1,a,2022-01-09 14:00:00,['o1'],,\"NA\"\n",
    "unclosed-quote": _H1 + "e1,a,2022-01-09 14:00:00,['o1'],,\"x\n",
    "duplicate-columns": "ocel:eid,ocel:activity,ocel:timestamp,ocel:type:a,ocel:type:a,x,x,\n"
                         "e1,a,2022-01-09 14:00:00,['o1'],['o2'],1,2,3\n",
    "type-prefix-twice": "ocel:eid,ocel:activity,ocel:timestamp,ocel:type:aocel:type:b\n"
                         "e1,a,2022-01-09 14:00:00,['o1']\n",
    "unrelated-without-id": _H1 + ",a,2022-01-09 14:00:00,,,\ne2,,2022-01-09 15:00:00,[],,\n",
    "ts-date-then-time": _H1 + "e1,a,2022-01-09,['o1'],,\ne2,a,2022-01-09 15:00:00,['o1'],,\n",
    "ts-offset-then-naive": _H1 + "e1,a,2022-01-09T14:00:00+01:00,['o1'],,\ne2,a,2022-01-09T15:00:00,['o1'],,\n",
    "list-forms": _H1 + "e1,a,2022-01-09 14:00:00,\"[('a'), ('b' 'c'),]  # note\",\"[\n 'd',  \\\n'e']\",\n"
                        "e2,a,2022-01-09 15:00:00,\"['\\q', '\\101', '\\x4a', b'\\x41', '\\\n']\",\"[R'\\'', Rb'f']\",\n",
    "list-binop": _H1 + "e1,a,2022-01-09 14:00:00,\"[1+2]\",\"['a'] + ['b']\",\n",
    "list-complex": _H1 + "e1,a,2022-01-09 14:00:00,\"[1+2j]\",,\n",
    "list-fstring": _H1 + "e1,a,2022-01-09 14:00:00,\"[f'a']\",\"['a' b'b']\",\n",
    "list-tuple": _H1 + "e1,a,2022-01-09 14:00:00,\"[('a',)]\",,\n",
    "list-bad-bytes": _H1 + "e1,a,2022-01-09 14:00:00,\"[b'\\xff']\",,\n",
    "list-surrogate": _H1 + "e1,a,2022-01-09 14:00:00,\"['\\ud800']\",,\n",
    "same-id-two-types": _H1 + "e1,a,2022-01-09 14:00:00,['x'],['x'],\n",
    "missing-columns": "ocel:eid,ocel:type:a\ne1,['o1']\n",
}

# OCEL 2.0 CSV texts: each of the reader's checks, references and escapes,
# object rows and type inference.
_H2 = "id,activity,timestamp,ot:a,ot:b,x\r\n"
_CSV2_TEXTS = {
    "ok": _H2 + "e1,go,2022-01-09T14:00:00Z,a1#q,b1,7\r\n",
    "header-only": _H2,
    "empty": "",
    "no-timezone": _H2 + "e1,go,2022-01-09T14:00:00,a1,,\r\n",
    "ts-forms": _H2 + "e1,go,2022-01-09 14:00:00Z,a1,,\r\ne2,go,2022-01-09T14:00Z,a1,,\r\n"
                      "e3,go,2022-01-09T14:00:00.123456789Z,a1,,\r\ne4,go,2022-01-09T15:00:00+0100,a1,,\r\n"
                      "e5,go,2022-01-09T14:00:00-00:30,a1,,\r\n",
    "ts-date-only": _H2 + "e1,go,2022-01-09Z,a1,,\r\n",
    "ts-garbage": _H2 + "e1,go,soon Z,a1,,\r\n",
    "ts-sort": _H2 + "e2,go,2022-01-09T15:00:00Z,a1,,\r\ne1,go,2022-01-09T14:00:00Z,a1,,\r\ne3,go,2022-01-09T14:00:00Z,a2,,\r\n",
    "duplicate-event": _H2 + "e1,go,2022-01-09T14:00:00Z,a1,,\r\ne1,go,2022-01-09T15:00:00Z,a1,,\r\n",
    "id-without-activity": _H2 + "e1,,2022-01-09T14:00:00Z,a1,,\r\n",
    "activity-without-id": _H2 + ",go,2022-01-09T14:00:00Z,a1,,\r\n",
    "attribute-in-object-row": _H2 + ",,,a1,,7\r\n",
    "o2o-undeclared": _H2 + "a1,o2o,,a2,,\r\n",
    "o2o-no-target": _H2 + "e1,go,2022-01-09T14:00:00Z,a1,,\r\na1,o2o,,,,\r\n",
    "o2o-json-no-time": _H2 + "e1,go,2022-01-09T14:00:00Z,a1,,\r\na1,o2o,,\"a2{\"\"k\"\":1}\",,\r\n",
    "o2o-json-time": _H2 + "e1,go,2022-01-09T14:00:00Z,a1,,\r\na1,o2o,2022-01-09T15:00:00Z,\"a2#q{\"\"k\"\":1}\",b1,\r\n",
    "o2o-duplicate": _H2 + "e1,go,2022-01-09T14:00:00Z,a1,,\r\na1,o2o,,a2/a2,,\r\n",
    "attribute-row-without-json": _H2 + ",,2022-01-09T14:00:00Z,a1,,\r\n",
    "attribute-row-without-reference": _H2 + ",,2022-01-09T14:00:00Z,,,\r\n",
    "declaration-with-qualifier": _H2 + ",,,a1#q,,\r\n",
    "declaration-empty": _H2 + ",,,,,\r\n",
    "unescaped-hash": _H2 + "e1,go,2022-01-09T14:00:00Z,a1#q#r,,\r\n",
    "unescaped-brace-in-id": _H2 + "e1,go,2022-01-09T14:00:00Z,a}1,,\r\n",
    "bad-escape": _H2 + "e1,go,2022-01-09T14:00:00Z,a\\x1,,\r\n",
    "dangling-escape": _H2 + "e1,go,2022-01-09T14:00:00Z,a1\\,,\r\n",
    "escapes": _H2 + "e1,go,2022-01-09T14:00:00Z,a\\/1#q\\#\\{x\\\\/a\\{2,,\r\n",
    "empty-reference": _H2 + "e1,go,2022-01-09T14:00:00Z,a1//a2,,\r\n",
    "empty-id": _H2 + "e1,go,2022-01-09T14:00:00Z,#q,,\r\n",
    "spaces": _H2 + " e1 , go , 2022-01-09T14:00:00Z , a1 # q ,\" b1 {\"\"k\"\": 1} \", x \r\n",
    "empty-qualifier": _H2 + "e1,go,2022-01-09T14:00:00Z,a1#,,\r\n",
    "json-not-object": _H2 + "e1,go,2022-01-09T14:00:00Z,\"a1{\"\"k\"\"}\",,\r\n",
    "json-array": _H2 + "e1,go,2022-01-09T14:00:00Z,\"a1{\"\"k\"\":[1]}\",,\r\n",
    "json-nan": _H2 + "e1,go,2022-01-09T14:00:00Z,\"a1{\"\"k\"\":NaN}\",,\r\n",
    "json-slash-brace": _H2 + "e1,go,2022-01-09T14:00:00Z,\"a1{\"\"k\"\":\"\"x/y}\"\"}/a2{\"\"k\"\":\"\"\\\\\"\"\"\"}\",,\r\n",
    "relation-duplicate": _H2 + "e1,go,2022-01-09T14:00:00Z,a1#q/a1#q,,\r\n",
    "relation-qualifiers": _H2 + "e1,go,2022-01-09T14:00:00Z,a1#q/a1#r/a1,,\r\n",
    "two-types": _H2 + "e1,go,2022-01-09T14:00:00Z,x1,x1,\r\n",
    "conflict": _H2 + "e1,go,2022-01-09T14:00:00Z,\"a1{\"\"k\"\":1}\",,\r\ne2,go,2022-01-09T14:00:00Z,\"a1{\"\"k\"\":2}\",,\r\n",
    "same-value-twice": _H2 + "e1,go,2022-01-09T14:00:00Z,\"a1{\"\"k\"\":1}\",,\r\n"
                              "e2,go,2022-01-09T14:00:00Z,\"a1{\"\"k\"\":1.0}\",,\r\n",
    "epoch-conflict": _H2 + ",,,\"a1{\"\"k\"\":1}\",,\r\n,,1970-01-01T00:00:00Z,\"a1{\"\"k\"\":2}\",,\r\n",
    "blank-line": _H2 + "e1,go,2022-01-09T14:00:00Z,a1,,\r\n\r\n",
    "ragged": _H2 + "e1,go,2022-01-09T14:00:00Z,a1\r\n",
    "duplicate-header": "id,activity,timestamp,ot:a,ot:a\r\n",
    "bad-quote": _H2 + "e1,go,2022-01-09T14:00:00Z,\"a1\"x,,\r\n",
    "unclosed-quote": _H2 + "e1,go,2022-01-09T14:00:00Z,\"a1,,\r\n",
    "missing-column": "id,activity,ot:a\r\ne1,go,a1\r\n",
    "type-prefix-only": "id,activity,timestamp,ot:\r\n",
    "bom": "﻿" + _H2 + "e1,go,2022-01-09T14:00:00Z,a1,,\r\n",
    "lf": _H2.replace("\r\n", "\n") + "e1,go,2022-01-09T14:00:00Z,a1,,\"x\ny\"\n",
    "infer": "id,activity,timestamp,ot:a,i,f,b,t,s,big,lead,exp,neg,nan,mixed\r\n"
             "e1,go,2022-01-09T14:00:00Z,a1,1,1,true,2022-01-09T14:00:00Z,x,9223372036854775807,007,1e5,-0,nan,1\r\n"
             "e2,go,2022-01-09T15:00:00Z,a1,-2,2.5,False,2022-01-09T16:00:00+01:00,,9223372036854775808,1.50,1.0,-1,1,true\r\n"
             "e3,go,2022-01-09T16:00:00Z,a1,,0.1,TRUE,2022-01-09,y,1,2,2.0,0,2,2022-01-09T14:00:00Z\r\n",
    "infer-objects": "id,activity,timestamp,ot:a,ot:b\r\n"
                     "e1,go,2022-01-09T14:00:00Z,\"a1{\"\"n\"\":1,\"\"m\"\":\"\"2\"\",\"\"t\"\":\"\"2022-01-09T14:00:00Z\"\"}\","
                     "\"b1{\"\"n\"\":true,\"\"m\"\":1.5}\"\r\n"
                     ",,2022-01-09T15:00:00Z,\"a1{\"\"n\"\":2.5,\"\"m\"\":\"\"3\"\",\"\"t\"\":\"\"x\"\"}\",\"b1{\"\"n\"\":\"\"False\"\"}\"\r\n"
                     ",,,\"a2{\"\"m\"\":4,\"\"u\"\":null}\",\r\n",
}


def _csv_cases(texts, read, suffix):
    return {"cases": {name: _read_text(read, text, suffix) | {"text": text}
                      for name, text in texts.items()}}


_OBJECTS_LOG = (
    "ocel:eid,ocel:activity,ocel:timestamp,ocel:type:order,ocel:type:item\n"
    "e1,place,2022-01-09 14:00:00,['o1'],\"['i1', 'i2']\"\n"
    "e2,pick,2022-01-09 15:00:00,,['i1']\n"
)

# OCEL 1.0 CSV objects tables, each read with ``_OBJECTS_LOG``. pm4py does
# not check the objects table against the relations.
_CSV_OBJECTS_TEXTS = {
    "basic": "ocel:oid,ocel:type,price\no1,order,12.5\ni1,item,\ni2,item,3\n",
    "no-type-column": "ocel:oid,price\no1,12.5\ni1,\n",
    "no-oid-column": "ocel:type,price\norder,12.5\n",
    "repeated-object": "ocel:oid,ocel:type\no1,order\no1,item\ni1,item\n",
    "unlisted-object": "ocel:oid,ocel:type\no1,order\n",
    "other-type": "ocel:oid,ocel:type\no1,item\ni1,order\n",
    "na-id": "ocel:oid,ocel:type\nNA,order\no1,order\n,item\n",
    "na-type": "ocel:oid,ocel:type\no1,NA\ni1,\ni2,item\n",
    "extra-columns-only": "ocel:oid,ocel:type,a,b\no1,order,,\n",
    "empty": "ocel:oid,ocel:type\n",
}


@case("read-csv-objects-texts", functions=["pm4py.read_ocel_csv"])
def _csv_objects_text_cases(fixtures):
    import tempfile

    out = {}
    for name, text in _CSV_OBJECTS_TEXTS.items():
        with tempfile.TemporaryDirectory() as tmp:
            log, objects = Path(tmp) / "log.csv", Path(tmp) / "objects.csv"
            log.write_text(_OBJECTS_LOG, encoding="utf-8")
            objects.write_text(text, encoding="utf-8")
            try:
                result = {"ocel": _tables(pm4py.read_ocel_csv(str(log), str(objects)))}
            except Exception as e:
                result = {"error": type(e).__name__}
        out[name] = result | {"objects_text": text}
    return {"log": _OBJECTS_LOG, "cases": out}


@case("read-csv-texts", functions=["pm4py.read_ocel_csv"])
def _csv_text_cases(fixtures):
    return _csv_cases(_CSV_TEXTS, pm4py.read_ocel_csv, ".csv")


@case("read-csv2-texts", functions=["pm4py.read_ocel2_csv"])
def _csv2_text_cases(fixtures):
    return _csv_cases(_CSV2_TEXTS, pm4py.read_ocel2_csv, ".ocel.csv")


# The writers, keyed by the name the golden uses. ``json`` is pm4py's
# ``write_ocel_json``, which picks the ``ocel20`` variant for a log with OCEL
# 2.0 features and ``classic`` otherwise.
_WRITERS = {
    "json": ("pm4py.write_ocel_json", "jsonocel", pm4py.write_ocel_json),
    "xml": ("pm4py.write_ocel_xml", "xmlocel", pm4py.write_ocel_xml),
    "json2": ("pm4py.write_ocel2_json", "jsonocel", pm4py.write_ocel2_json),
    "xml2": ("pm4py.write_ocel2_xml", "xmlocel", pm4py.write_ocel2_xml),
    "csv2": ("pm4py.write_ocel2_csv", "ocel.csv", pm4py.write_ocel2_csv),
}


def _write_csv(ocel, path):
    """pm4py's ``write_ocel_csv`` with an objects file; returns its text."""
    objects = path.with_name("objects.csv")
    pm4py.write_ocel_csv(ocel, str(path), str(objects))
    return objects.read_bytes().decode("utf-8")


_WRITER_FUNCTIONS = [w[0] for w in _WRITERS.values()] + ["pm4py.write_ocel_csv"]


def write_all(ocel):
    """Writes ``ocel`` with each writer and records the file text, or the
    error when pm4py raises. ``input`` holds the tables the writers start
    from, in the form ``summarize`` uses, and ``globals`` the log's globals."""
    import copy
    import tempfile

    # pm4py's writers change the log they are given (the consistency step
    # fills in qualifiers), so record the input first and give each writer
    # its own copy.
    tables = _tables(ocel)
    out = {}
    with tempfile.TemporaryDirectory() as tmp:
        for name, (_, extension, write) in _WRITERS.items():
            path = Path(tmp) / f"{name}.{extension}"
            try:
                write(copy.deepcopy(ocel), str(path))
                out[name] = {"text": path.read_bytes().decode("utf-8")}
            except Exception as e:
                out[name] = {"error": type(e).__name__}
        path = Path(tmp) / "csv.csv"
        try:
            objects = _write_csv(copy.deepcopy(ocel), path)
            out["csv"] = {"text": path.read_bytes().decode("utf-8"), "objects": objects}
        except Exception as e:
            out["csv"] = {"error": type(e).__name__}
    result = {"input": tables, "globals": ocel.globals, "writers": out}
    times = ocel.events[ocel.event_timestamp]
    if len(times) and getattr(times.dt, "tz", None) is None:
        # The tables hold the times as UTC; pm4py's own times are naive.
        result["naive_times"] = True
    return result


def _write_pinned(rel, path):
    """Reads ``path`` like ``_read_pinned``, then writes it with each writer."""
    golden_tools = str(Path(__file__).resolve().parent.parent)
    env = dict(os.environ, PYTHONHASHSEED="0")
    env["PYTHONPATH"] = os.pathsep.join(filter(None, [golden_tools, env.get("PYTHONPATH")]))
    out = subprocess.run([sys.executable, __file__, "--write", rel, str(path)], env=env,
                         check=True, capture_output=True, text=True)
    return json.loads(out.stdout)


# pm4py's OCEL 1.0 CSV reader orders objects by set iteration, so its logs
# are left out.
_WRITTEN = [rel for rel in _READERS if not rel.endswith(".csv") or rel.endswith(".ocel.csv")]

for _rel in _WRITTEN:
    def _run_write(fixtures, _rel=_rel):
        return _write_pinned(_rel, fixtures["log"])

    case("write-" + _rel.replace(".", "-").replace("_", "-"), fixture="ocel/" + _rel,
         functions=[_READERS[_rel][0]] + _WRITER_FUNCTIONS)(_run_write)


@case("write-typed-csv-objects",
      fixtures={"log": "ocel/typed.csv", "objects": "ocel/typed-objects.csv"},
      functions=["pm4py.read_ocel_csv"] + _WRITER_FUNCTIONS)
def _write_typed_csv_objects(fixtures):
    # An OCEL 1.0 CSV log with naive times, read with an objects file so
    # that the object order is fixed.
    return write_all(pm4py.read_ocel_csv(str(fixtures["log"]), str(fixtures["objects"])))


@case("write-empty", functions=_WRITER_FUNCTIONS)
def _write_empty(fixtures):
    return write_all(pm4py.OCEL())


def _synthetic(ocel20, ghosts=True):
    """A small log built from data frames. Its attribute columns are in
    name order, so a writer that orders attributes by name gives pm4py's
    bytes. It covers escaping, Python float text, typed and mixed columns,
    rows that the relation filter drops, and, with ``ocel20``, qualifiers,
    object-to-object relations and object changes."""
    def ts(text):
        return pd.Timestamp(text, tz="UTC") if text else pd.NaT

    events = pd.DataFrame({
        "ocel:eid": ["e1", "e2", "e3", "e4", "e5"],
        "ocel:activity": ["place order", "pick", "pick", "ship & <go>", "audit"],
        "ocel:timestamp": [ts("2022-01-01T10:00:00.25"), ts("2022-01-01T11:00:00"),
                           ts("2022-01-02T00:00:00"), ts("2022-01-03T00:00:00"),
                           ts("2022-01-04T00:00:00")],
        "a_float": [1e16, 1.5e-7, 0.0001, 123456789.125, 2.0],
        "b_int": [3, -4, 5, 0, 1],
        "c_text": ['\u00dcn\u00efc\u00f6d\u00e9 & <tag> "q" \'s\'\n\ttab\r', "\u65e5\u672c \U0001f600 \u2028",
                   None, "", "unused"],
        "d_date": [ts("2022-01-02T03:04:05.5"), ts(None), ts("2022-01-03T00:00:00"), ts(None),
                   ts(None)],
        "e_mixed": ["x", 2.5, 7, None, "y"],
        "f_bool": [True, False, True, False, True],
    })
    objects = pd.DataFrame({
        "ocel:oid": ["o1", "i<1>", "i2", "lonely"],
        "ocel:type": ["order", "it\u20acm", "it\u20acm", "it\u20acm"],
        "g_price": [12.0, float("nan"), 0.1, 1.0],
        "h_since": [ts("2021-12-31T23:00:00"), ts(None), ts(None), ts(None)],
        "i_label": ["big", 'a"b', None, "alone"],
    })
    pairs = [("e1", "o1", "placer"), ("e1", "i<1>", None), ("e1", "i2", "item"),
             ("e2", "i<1>", "picked"), ("e3", "i2", "picked"), ("e4", "o1", ""),
             ("e4", "ghost", "lost")]
    if not ghosts:
        pairs = [p for p in pairs if p[1] != "ghost"]
    ev = events.set_index("ocel:eid")
    ob = dict(zip(objects["ocel:oid"], objects["ocel:type"]))
    relations = pd.DataFrame({
        "ocel:eid": [e for e, _, _ in pairs],
        "ocel:activity": [ev.loc[e, "ocel:activity"] for e, _, _ in pairs],
        "ocel:timestamp": [ev.loc[e, "ocel:timestamp"] for e, _, _ in pairs],
        "ocel:oid": [o for _, o, _ in pairs],
        "ocel:type": [ob.get(o, "it\u20acm") for _, o, _ in pairs],
    })
    if not ocel20:
        globals_ = {"ocel:global-event": {"ocel:activity": "none"},
                    "ocel:global-object": {"ocel:type": "none"}}
        return pm4py.OCEL(events=events, objects=objects, relations=relations, globals=globals_)
    relations["ocel:qualifier"] = [q for _, _, q in pairs]
    o2o = pd.DataFrame({"ocel:oid": ["o1", "o1", "i2"], "ocel:oid_2": ["i<1>", "i2", "ghost"],
                        "ocel:qualifier": ["contains", None, "x"]})
    if not ghosts:
        o2o = o2o[o2o["ocel:oid_2"] != "ghost"]
    e2e = pd.DataFrame({"ocel:eid": ["e1"], "ocel:eid_2": ["e2"], "ocel:qualifier": ["next"]})
    changes = pd.DataFrame({
        "ocel:oid": ["o1", "i2", "i<1>", "lonely"],
        "ocel:type": ["order", "it\u20acm", "it\u20acm", "it\u20acm"],
        "ocel:timestamp": [ts("2022-01-02T00:00:00.5"), ts("2022-01-03T00:00:00"),
                           ts("2022-01-03T00:00:00"), ts("2022-01-03T00:00:00")],
        "ocel:field": ["g_price", "i_label", "g_price", "g_price"],
        "g_price": [13.5, float("nan"), float("nan"), 2.0],
        "i_label": [None, "relabelled", None, None],
    })
    return pm4py.OCEL(events=events, objects=objects, relations=relations, o2o=o2o, e2e=e2e,
                      object_changes=changes)


@case("write-synthetic", functions=["pm4py.OCEL"] + _WRITER_FUNCTIONS)
def _write_synthetic(fixtures):
    return write_all(_synthetic(False))


@case("write-synthetic20", functions=["pm4py.OCEL"] + _WRITER_FUNCTIONS)
def _write_synthetic20(fixtures):
    return write_all(_synthetic(True))



@case("write-synthetic20-linked", functions=["pm4py.OCEL"] + _WRITER_FUNCTIONS)
def _write_synthetic20_linked(fixtures):
    # Without relations to unknown objects, which pm4py's OCEL 2.0 CSV
    # writer refuses.
    return write_all(_synthetic(True, ghosts=False))


if __name__ == "__main__":
    if sys.argv[1] == "--write":
        print(json.dumps(write_all(_READERS[sys.argv[2]][1](sys.argv[3]))))
    else:
        print(json.dumps(summarize(_READERS[sys.argv[1]][1](sys.argv[2]))))
