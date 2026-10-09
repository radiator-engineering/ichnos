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
    objects = [{"id": str(r[oid]), "type": str(r[otype]), "attributes": _attributes(r, {oid, otype})}
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


# The writers, keyed by the name the golden uses. ``json`` is pm4py's
# ``write_ocel_json``, which picks the ``ocel20`` variant for a log with OCEL
# 2.0 features and ``classic`` otherwise.
_WRITERS = {
    "json": ("pm4py.write_ocel_json", "jsonocel", pm4py.write_ocel_json),
    "xml": ("pm4py.write_ocel_xml", "xmlocel", pm4py.write_ocel_xml),
    "json2": ("pm4py.write_ocel2_json", "jsonocel", pm4py.write_ocel2_json),
    "xml2": ("pm4py.write_ocel2_xml", "xmlocel", pm4py.write_ocel2_xml),
}


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
    return {"input": tables, "globals": ocel.globals, "writers": out}


def _write_pinned(rel, path):
    """Reads ``path`` like ``_read_pinned``, then writes it with each writer."""
    golden_tools = str(Path(__file__).resolve().parent.parent)
    env = dict(os.environ, PYTHONHASHSEED="0")
    env["PYTHONPATH"] = os.pathsep.join(filter(None, [golden_tools, env.get("PYTHONPATH")]))
    out = subprocess.run([sys.executable, __file__, "--write", rel, str(path)], env=env,
                         check=True, capture_output=True, text=True)
    return json.loads(out.stdout)


_WRITTEN = [rel for rel in _READERS if not rel.endswith(".csv")]

for _rel in _WRITTEN:
    def _run_write(fixtures, _rel=_rel):
        return _write_pinned(_rel, fixtures["log"])

    case("write-" + _rel.replace(".", "-").replace("_", "-"), fixture="ocel/" + _rel,
         functions=[_READERS[_rel][0]] + [w[0] for w in _WRITERS.values()])(_run_write)


@case("write-empty", functions=[w[0] for w in _WRITERS.values()])
def _write_empty(fixtures):
    return write_all(pm4py.OCEL())


def _synthetic(ocel20):
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


@case("write-synthetic", functions=["pm4py.OCEL"] + [w[0] for w in _WRITERS.values()])
def _write_synthetic(fixtures):
    return write_all(_synthetic(False))


@case("write-synthetic20", functions=["pm4py.OCEL"] + [w[0] for w in _WRITERS.values()])
def _write_synthetic20(fixtures):
    return write_all(_synthetic(True))


if __name__ == "__main__":
    if sys.argv[1] == "--write":
        print(json.dumps(write_all(_READERS[sys.argv[2]][1](sys.argv[3]))))
    else:
        print(json.dumps(summarize(_READERS[sys.argv[1]][1](sys.argv[2]))))
