import copy
import json
from pathlib import Path
import unittest

import content_addressable as addressing

from consumer import Dial, check_artifact, read_artifact
from consumer import main, portable, property_outcome, NAMES

FIXTURE = Path(__file__).resolve().parents[1] / "fixtures" / "dial.json"


class WrongFlow(Dial):
    def handle(self, key):
        flow = super().handle(key)
        if flow["kind"] == "close":
            flow["applied"] = not flow["applied"]
        return flow


class WrongIntent(Dial):
    def intent(self):
        return None


class BooleanIntent(Dial):
    def intent(self):
        intent = super().intent()
        if self.accepted in (0, 1):
            return {"accepted": bool(self.accepted)}
        return intent


class ConsumerTests(unittest.TestCase):
    def setUp(self):
        self.artifact = read_artifact(FIXTURE.read_bytes())

    def test_the_real_python_component_is_driven_and_canonical_bytes_match(self):
        created = []

        def factory(seed):
            dial = Dial(seed)
            created.append(dial)
            return dial

        self.assertEqual(check_artifact(self.artifact, factory), 36)
        self.assertEqual(len(created), 36)
        canonical = addressing.to_canonical_dagcbor(self.artifact["corpus"])
        self.assertEqual(canonical, FIXTURE.with_suffix(".cbor").read_bytes())
        self.assertEqual(str(addressing.content_id(self.artifact["corpus"])), self.artifact["id"])

    def test_wrong_consumers_fail_even_when_all_their_views_match(self):
        for factory, field in [(WrongFlow, "flow"), (WrongIntent, "intent"), (BooleanIntent, "intent")]:
            with self.subTest(factory=factory.__name__), self.assertRaisesRegex(ValueError, field):
                check_artifact(self.artifact, factory)

    def test_tampered_content_and_validly_addressed_malformed_domains_are_refused(self):
        changed = copy.deepcopy(self.artifact)
        changed["corpus"]["seed"]["level"] = 1
        with self.assertRaises(ValueError):
            read_artifact(json.dumps(changed).encode())
        for mutate in [
            lambda c: c.update(schema="newtui.behavior-corpus/v99"),
            lambda c: c["domain"].update(alphabet=[{"kind": "left"}] * 2),
            lambda c: c["exploration"].update(transitions=1),
            lambda c: c.update(events=[]),
            lambda c: c["properties"][0].update(applicable=0),
        ]:
            changed = copy.deepcopy(self.artifact)
            mutate(changed["corpus"])
            changed["id"] = str(addressing.content_id(changed["corpus"]))
            with self.assertRaises(ValueError):
                read_artifact(json.dumps(changed).encode())

    def test_duplicate_keys_lossy_numbers_and_unknown_fields_are_refused(self):
        raw = json.dumps(self.artifact)
        for replacement in ['{"level":0,"level":1,"maximum":3}', '{"level":0.0,"maximum":3}', '{"level":9223372036854775808,"maximum":3}']:
            changed = raw.replace('"seed": {"level": 0, "maximum": 3}', '"seed": ' + replacement)
            self.assertNotEqual(raw, changed)
            with self.assertRaises(ValueError):
                read_artifact(changed.encode())
        changed = copy.deepcopy(self.artifact)
        changed["extra"] = True
        with self.assertRaises(ValueError):
            read_artifact(json.dumps(changed).encode())

    def test_state_cap_cannot_hide_transitions_explored_beyond_the_bound(self):
        changed = copy.deepcopy(self.artifact)
        changed["corpus"]["domain"]["max_states"] = 4
        changed["corpus"]["exploration"].update(exhausted=False, verdict="incomplete", incomplete_reason="state cap reached")
        changed["id"] = str(addressing.content_id(changed["corpus"]))
        with self.assertRaises(ValueError):
            read_artifact(json.dumps(changed).encode())

    def test_portable_depth_is_relative_to_seed_or_intent_not_the_envelope(self):
        nested = 0
        for _ in range(32):
            nested = [nested]
        changed = copy.deepcopy(self.artifact)
        changed["corpus"]["seed"] = nested
        changed["id"] = str(addressing.content_id(changed["corpus"]))
        read_artifact(json.dumps(changed).encode())
        with self.assertRaises(ValueError):
            portable([nested])
        for bad in [1.5, 1 << 63, object(), {1: "not a string key"}]:
            with self.assertRaises(ValueError):
                portable(bad)
        for valid in [-(1 << 63), (1 << 63) - 1, {"/": "ordinary map"}, "🙂"]:
            portable(valid)
        with self.assertRaises(UnicodeError):
            portable("\ud800")

    def test_cli_drives_the_native_consumer_and_rejects_bad_usage(self):
        from contextlib import redirect_stdout
        import io
        output = io.StringIO()
        with redirect_stdout(output):
            main([str(FIXTURE)])
        self.assertIn("36 observations", output.getvalue())
        with self.assertRaises(ValueError):
            main([])
        with self.assertRaises(OSError):
            main([str(FIXTURE) + ".missing"])

    def test_actual_property_judgments_cover_failure_and_inapplicability(self):
        view = Dial({"level": 0, "maximum": 3}).view()
        changed = copy.deepcopy(view)
        changed["rows"][0]["selected"] = False
        self.assertEqual(property_outcome(NAMES[0], "state", view, {}, changed, {})["kind"], "violated")
        self.assertEqual(property_outcome(NAMES[0], "state", view, {}, {"rows": []}, {})["kind"], "not_applicable")
        for flow in [{"kind": "stay"}, {"kind": "close", "applied": True}]:
            self.assertEqual(property_outcome(NAMES[1], "transition", view, {"kind": "esc"}, view, flow)["kind"], "violated")
        view["rows"][0]["adjustable"] = False
        changed["rows"][0]["value"] = "99"
        self.assertEqual(property_outcome(NAMES[2], "transition", view, {"kind": "left"}, changed, {})["kind"], "violated")

    def test_factory_and_key_mutation_cannot_rewrite_the_evidence(self):
        original = copy.deepcopy(self.artifact)

        class MutatingDial(Dial):
            def __init__(self, seed):
                super().__init__(seed)
                seed.clear()

            def handle(self, key):
                result = super().handle(key)
                key.clear()
                return result

        self.assertEqual(check_artifact(self.artifact, MutatingDial), 36)
        self.assertEqual(self.artifact, original)

    def test_view_cannot_rewrite_the_flow_already_returned_by_handle(self):
        class SharedFlow(Dial):
            def __init__(self, seed):
                super().__init__(seed)
                self.flow = None

            def handle(self, key):
                self.flow = super().handle(key)
                return self.flow

            def view(self):
                if self.flow is not None and self.flow["kind"] == "close":
                    self.flow["applied"] = True
                return super().view()

        self.assertEqual(check_artifact(self.artifact, SharedFlow), 36)

    def test_reused_view_storage_does_not_rewrite_the_departure_snapshot(self):
        artifact = copy.deepcopy(self.artifact)
        corpus = artifact["corpus"]
        # A diagnostic corpus for a faulty fixed row: its first changing Right
        # violates the only property, which then retires by position.
        total = dict(name=NAMES[2], observations=0, applicable=0, held=0, outcome={"kind": "not_applicable"})
        for event in corpus["events"]:
            snapshots = [event["trace"]["initial"]] + [s["after"] for s in event["trace"]["steps"]]
            for snapshot in snapshots:
                snapshot["view"]["rows"][0]["adjustable"] = False
            event["checks"] = []
            if total["outcome"]["kind"] == "violated":
                continue
            steps = event["trace"]["steps"]
            before = snapshots[-2]["view"] if steps else snapshots[0]["view"]
            key = steps[-1]["key"] if steps else {"kind": "other"}
            outcome = property_outcome(NAMES[2], event["kind"], before, key, snapshots[-1]["view"], {})
            event["checks"] = [{"property": 0, "outcome": outcome}]
            total["observations"] += 1
            if outcome["kind"] != "not_applicable":
                total["applicable"] += 1
                total["held"] += outcome["kind"] == "held"
                total["outcome"] = outcome
        self.assertEqual(total["outcome"]["kind"], "violated")
        corpus["properties"] = [total]
        corpus["exploration"]["verdict"] = "violated"
        artifact["id"] = str(addressing.content_id(corpus))

        class ReusedView(Dial):
            def __init__(self, seed):
                super().__init__(seed)
                self.live_view = super().view()
                self.live_view["rows"][0]["adjustable"] = False

            def view(self):
                self.live_view["rows"][0]["value"] = str(self.level)
                return self.live_view

        # It must reproduce the violation, then refuse the diagnostic verdict.
        # An aliased before-view instead judges Held and fails earlier.
        with self.assertRaisesRegex(ValueError, "contains violated properties"):
            check_artifact(artifact, ReusedView)


if __name__ == "__main__":
    unittest.main()
