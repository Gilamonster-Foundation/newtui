"""Independent Python dial consumer; content-addressable owns bytes and CIDs."""

import copy
import json
from pathlib import Path
import sys

import content_addressable as addressing

SCHEMA = "newtui.behavior-corpus/v1"
NAMES = ("selection is always in range", "escape always closes without applying", "only adjustable rows move")
KEYS = set("up down left right enter esc backspace tab back_tab home end page_up page_down other".split())


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def fields(value, names):
    require(type(value) is dict and set(value) == set(names.split()), "unknown or missing object fields")


def integer(value, limit=(1 << 63) - 1):
    require(type(value) is int and 0 <= value <= limit, "invalid bounded integer")


def portable(value, depth=0, limit=32):
    require(depth <= limit, "portable value nesting limit exceeded")
    if type(value) is str:
        value.encode("utf-8")
    elif type(value) is int:
        require(-(1 << 63) <= value < (1 << 63), "integer exceeds signed 64-bit range")
    elif value is None or type(value) is bool:
        pass
    elif type(value) is list:
        for item in value:
            portable(item, depth + 1, limit)
    elif type(value) is dict:
        for key, item in value.items():
            require(type(key) is str, "portable map keys must be strings")
            key.encode("utf-8")
            portable(item, depth + 1, limit)
    else:
        raise ValueError("unsupported portable value type")


def same(left, right):
    # Python considers True == 1. Observable intents must retain that type
    # distinction, so compare through the already-shipped canonical codec.
    return addressing.to_canonical_dagcbor(left) == addressing.to_canonical_dagcbor(right)


def input_key(value):
    require(type(value) is dict, "invalid key")
    kind = value.get("kind")
    require(type(kind) is str, "input key kind must be a string")
    if kind in KEYS:
        fields(value, "kind")
        return (kind,)
    require(kind in ("char", "ctrl"), "unknown input key")
    fields(value, "kind value")
    require(type(value["value"]) is str and len(value["value"]) == 1, "a character key needs one Unicode scalar")
    value["value"].encode("utf-8")
    return kind, value["value"]


def flow_shape(flow):
    require(type(flow) is dict, "invalid flow")
    if flow.get("kind") == "stay":
        fields(flow, "kind")
    else:
        fields(flow, "kind applied")
        require(flow["kind"] == "close" and type(flow["applied"]) is bool, "invalid close flow")


def outcome_shape(outcome):
    require(type(outcome) is dict, "invalid property outcome")
    if outcome.get("kind") in ("held", "not_applicable"):
        fields(outcome, "kind")
    else:
        fields(outcome, "kind detail")
        require(outcome["kind"] == "violated" and type(outcome["detail"]) is str, "invalid violation")


def snapshot_shape(snapshot):
    fields(snapshot, "view intent")
    portable(snapshot["intent"])
    view = snapshot["view"]
    fields(view, "title rows footer")
    require(type(view["title"]) is str and type(view["footer"]) is str and type(view["rows"]) is list, "invalid view")
    for row in view["rows"]:
        fields(row, "label value note selected adjustable")
        require(all(type(row[key]) is str for key in ("label", "value", "note")), "invalid row text")
        require(type(row["selected"]) is bool and type(row["adjustable"]) is bool, "invalid row flags")


def validate(corpus):
    fields(corpus, "schema component seed domain exploration properties events")
    require(corpus["schema"] == SCHEMA, "unsupported corpus schema")
    require(type(corpus["component"]) is str and 0 < len(corpus["component"].encode()) <= 128, "invalid component contract")
    portable(corpus["seed"])
    domain = corpus["domain"]
    fields(domain, "alphabet max_states max_depth")
    integer(domain["max_states"], 50000)
    integer(domain["max_depth"], 64)
    require(type(domain["alphabet"]) is list and len(domain["alphabet"]) <= 64, "invalid alphabet")
    alphabet = [input_key(key) for key in domain["alphabet"]]
    require(len(set(alphabet)) == len(alphabet), "duplicate alphabet key")
    report = corpus["exploration"]
    fields(report, "states transitions terminal_states exhausted verdict incomplete_reason replay_issues")
    for name in ("states", "transitions", "terminal_states"):
        integer(report[name])
    require(type(report["exhausted"]) is bool and type(report["replay_issues"]) is list, "invalid exploration")
    properties, events = corpus["properties"], corpus["events"]
    require(type(properties) is list and len(properties) <= 64, "invalid property set")
    require(type(events) is list and 0 < len(events) <= 100000, "missing or excessive observations")
    totals = []
    for prop in properties:
        fields(prop, "name observations applicable held outcome")
        require(type(prop["name"]) is str, "invalid property name")
        for name in ("observations", "applicable", "held"):
            integer(prop[name])
        outcome_shape(prop["outcome"])
        totals.append(dict(name=prop["name"], observations=0, applicable=0, held=0, outcome={"kind": "not_applicable"}))
    opened, transitions, terminal, initial = {}, set(), 0, None
    for index, event in enumerate(events):
        fields(event, "kind trace checks")
        require(event["kind"] in ("state", "transition") and type(event["checks"]) is list, "invalid observation")
        trace = event["trace"]
        fields(trace, "initial steps")
        snapshot_shape(trace["initial"])
        require(type(trace["steps"]) is list and len(trace["steps"]) <= domain["max_depth"], "trace exceeds domain depth")
        if index == 0:
            require(event["kind"] == "state" and not trace["steps"], "the first observation must be the fresh seed")
            initial = trace["initial"]
            opened[()] = trace
        require(same(trace["initial"], initial), "a trace has a different initial state")
        path = []
        for at, step in enumerate(trace["steps"]):
            fields(step, "key flow after")
            path.append(input_key(step["key"]))
            require(path[-1] in alphabet, "key outside declared alphabet")
            flow_shape(step["flow"])
            snapshot_shape(step["after"])
            require(at + 1 == len(trace["steps"]) or step["flow"]["kind"] == "stay", "path continues after close")
        path = tuple(path)
        if index and event["kind"] == "transition":
            require(len(opened) < domain["max_states"], "transition recorded after the state cap")
            require(path and path not in transitions and path[:-1] in opened, "invalid transition departure")
            require(same(trace["steps"][:-1], opened[path[:-1]]["steps"]), "replay prefix mismatch")
            transitions.add(path)
            if trace["steps"][-1]["flow"]["kind"] == "close":
                require(index + 1 < len(events) and type(events[index + 1]) is dict and events[index + 1].get("kind") == "state" and same(events[index + 1].get("trace"), trace), "missing terminal state")
        elif index:
            require(events[index - 1]["kind"] == "transition" and same(events[index - 1]["trace"], trace), "state does not follow its transition")
            if trace["steps"][-1]["flow"]["kind"] == "stay":
                require(path not in opened, "duplicate open state path")
                opened[path] = trace
            else:
                terminal += 1
        expected = [i for i, total in enumerate(totals) if total["outcome"]["kind"] != "violated"]
        for result in event["checks"]:
            fields(result, "property outcome")
            integer(result["property"])
            outcome_shape(result["outcome"])
        require([result["property"] for result in event["checks"]] == expected, "property positions or retirement differ")
        for result in event["checks"]:
            total, outcome = totals[result["property"]], result["outcome"]
            total["observations"] += 1
            if outcome["kind"] != "not_applicable":
                total["applicable"] += 1
                total["held"] += outcome["kind"] == "held"
                total["outcome"] = outcome
    require(same(totals, properties) and (len(opened), len(transitions), terminal) == (report["states"], report["transitions"], report["terminal_states"]), "reported counts or outcomes differ")
    capped = len(opened) >= domain["max_states"] or any(len(path) >= domain["max_depth"] for path in opened)
    require(report["exhausted"] != capped, "exhaustion disagrees with bounds")
    for issue in report["replay_issues"]:
        fields(issue, "path reason")
        require(type(issue["path"]) is list, "invalid replay path")
        path = tuple(input_key(key) for key in issue["path"])
        require(path in opened, "replay issue names an unknown departure")
        reason = issue["reason"]
        require(type(reason) is dict, "invalid replay reason")
        if reason.get("kind") == "different_state":
            fields(reason, "kind")
        else:
            fields(reason, "kind at")
            integer(reason["at"])
            require(reason["kind"] == "closed_during_replay" and reason["at"] < len(path), "invalid replay-close index")
    if report["exhausted"] and not report["replay_issues"]:
        require(all(path + (key,) in transitions for path in opened for key in alphabet), "exhausted walk omits an outgoing edge")
    incomplete = not report["exhausted"] or bool(report["replay_issues"]) or not properties or not transitions or any(p["applicable"] == 0 for p in properties)
    verdict = "incomplete" if incomplete else "violated" if any(p["outcome"]["kind"] == "violated" for p in properties) else "clean"
    require(report["verdict"] == verdict, "verdict disagrees with evidence")
    reason = report["incomplete_reason"]
    require((type(reason) is str and bool(reason)) if incomplete else reason is None, "invalid incomplete explanation")


class Dial:
    def __init__(self, seed):
        fields(seed, "level maximum")
        integer(seed["maximum"], 20)
        integer(seed["level"], seed["maximum"])
        self.level = seed["level"]
        self.maximum = seed["maximum"]
        self.accepted = None

    def handle(self, key):
        kind = key["kind"]
        if kind == "right":
            self.level = min(self.level + 1, self.maximum)
        elif kind == "left":
            self.level = max(0, self.level - 1)
        elif kind == "enter":
            self.accepted = self.level
            return {"kind": "close", "applied": True}
        elif kind == "esc":
            return {"kind": "close", "applied": False}
        return {"kind": "stay"}

    def view(self):
        return {"title": "dial", "rows": [{"label": "level", "value": str(self.level),
                "note": "", "selected": True, "adjustable": True}], "footer": "arrows change"}

    def intent(self):
        return None if self.accepted is None else {"accepted": self.accepted}


def read_artifact(raw):
    require(len(raw) <= 8 * 1024 * 1024, "artifact exceeds 8 MiB")

    def unique(pairs):
        value = {}
        for key, item in pairs:
            require(key not in value, "duplicate JSON object key")
            value[key] = item
        return value

    def reject(_):
        raise ValueError("portable numbers must be signed 64-bit integers")

    artifact = json.loads(raw, object_pairs_hook=unique, parse_float=reject, parse_constant=reject)
    # Envelope/trace nesting is separate from each seed/intent's 32-level bound.
    portable(artifact, limit=80)
    fields(artifact, "id corpus")
    validate(artifact["corpus"])
    require(type(artifact["id"]) is str and str(addressing.content_id(artifact["corpus"])) == artifact["id"], "content identity mismatch")
    return artifact


def property_outcome(name, kind, before, key, after, flow):
    held, outside = {"kind": "held"}, {"kind": "not_applicable"}
    if name == NAMES[0]:
        if not after["rows"]:
            return outside
        count = sum(row["selected"] for row in after["rows"])
        if count == 1:
            return held
        detail = f"{len(after['rows'])} rows and nothing selected — a key would act on no row" if not count else f"{count} rows claim the cursor at once"
    elif name == NAMES[1]:
        if kind != "transition" or key["kind"] != "esc":
            return outside
        if flow == {"kind": "close", "applied": False}:
            return held
        detail = "escape closed the component AS AN APPLY" if flow["kind"] == "close" else "escape left the component open"
    elif name == NAMES[2]:
        if kind != "transition" or key["kind"] not in ("left", "right"):
            return outside
        selected = next((i for i, row in enumerate(before["rows"]) if row["selected"]), None)
        if selected is None or selected >= len(after["rows"]):
            return outside
        old, new = before["rows"][selected], after["rows"][selected]
        if old["adjustable"] or old["value"] == new["value"]:
            return held
        detail = f"`{old['label']}` is not adjustable but {key['kind'].title()} changed it from `{old['value']}` to `{new['value']}`"
    else:
        raise ValueError("unsupported property contract")
    return {"kind": "violated", "detail": detail}


def check_artifact(artifact, factory=Dial):
    # Revalidate in-memory callers too; reading a valid file is not a lifetime
    # guarantee that nobody has changed its mutable Python dictionaries.
    artifact = read_artifact(json.dumps(artifact, ensure_ascii=False).encode())
    corpus = artifact["corpus"]
    require(corpus["component"] == "newtui.example-dial/v1", "unsupported component contract")
    require(all(prop["name"] in NAMES for prop in corpus["properties"]), "unsupported property contract")
    for index, event in enumerate(corpus["events"]):
        component = factory(copy.deepcopy(corpus["seed"]))
        view = copy.deepcopy(component.view())
        require(same({"view": view, "intent": component.intent()}, event["trace"]["initial"]), f"observation {index}: initial snapshot mismatch")
        before, key, flow = view, {"kind": "other"}, {"kind": "stay"}
        for at, step in enumerate(event["trace"]["steps"]):
            before, key = view, step["key"]
            flow = copy.deepcopy(component.handle(copy.deepcopy(key)))
            require(same(flow, step["flow"]), f"observation {index}, key {at}: flow mismatch")
            view = copy.deepcopy(component.view())
            require(same(view, step["after"]["view"]), f"observation {index}, key {at}: view mismatch")
            require(same(component.intent(), step["after"]["intent"]), f"observation {index}, key {at}: intent mismatch")
        for result in event["checks"]:
            actual = property_outcome(corpus["properties"][result["property"]]["name"], event["kind"], before, key, view, flow)
            require(same(actual, result["outcome"]), f"observation {index}: property outcome mismatch")
    require(corpus["exploration"]["verdict"] == "clean", "matching evidence is incomplete or contains violated properties")
    return len(artifact["corpus"]["events"])


def main(args):
    require(len(args) == 1, "usage: consumer.py CORPUS.json")
    with Path(args[0]).open("rb") as stream:
        raw = stream.read(8 * 1024 * 1024 + 1)
    checked = check_artifact(read_artifact(raw))
    print(f"conformant: {checked} observations (independent Python component)")


if __name__ == "__main__":
    try:
        main(sys.argv[1:])
    except (ValueError, OSError, UnicodeError) as error:
        sys.exit(f"corpus: {error}")
