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


if __name__ == "__main__":
    print(json.dumps(summarize(_READERS[sys.argv[1]][1](sys.argv[2]))))
