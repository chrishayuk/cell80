"""The margin gate, offline: pure gate math over synthetic decisions (no embedding
model, no library) — answered/escalate accounting, per-split stats, and the
calibration sweep's operating-point rule."""

from cell_eval.tiers import Decision, TierReport, calibrate


def _d(cat, margin, top1_ok):
    top = [(0.9, "right" if top1_ok else "wrong"), (0.9 - margin, "other")]
    return Decision(query="q", expected=["right"], category=cat, top=top, margin=margin)


def _report(theta=0.1):
    r = TierReport(embed_model="synthetic", theta=theta)
    # direct: confident and right
    r.decisions += [_d("direct", 0.3, True), _d("direct", 0.25, True)]
    # paraphrase: one confident-right, one shaky-wrong (must escalate at θ=0.1)
    r.decisions += [_d("paraphrase", 0.2, True), _d("paraphrase", 0.02, False)]
    # adversarial: confident-wrong at low margin, escalates once θ clears it
    r.decisions += [_d("adversarial", 0.05, False), _d("adversarial", 0.01, False)]
    for i in range(len(r.decisions)):
        r.tier1_top[i] = r.decisions[i].top[0][1]
    return r


def test_gate_answers_confident_and_escalates_shaky():
    r = _report(theta=0.1)
    assert r.split("direct").answer_rate == 1.0
    assert r.split("direct").precision_on_answered == 1.0
    p = r.split("paraphrase")
    assert p.answered == 1 and p.answered_correct == 1  # the shaky-wrong escalated
    a = r.split("adversarial")
    assert a.answered == 0  # everything below the margin escalates


def test_calibration_picks_the_smallest_safe_theta():
    r = _report()
    cal = calibrate(r, floor=0.75)
    # Adversarial answers are all wrong below 0.06 margin; the smallest θ that clears
    # the floor is the first one where no adversarial query is answered (vacuous 1.0).
    assert cal["chosen_theta"] is not None
    t = cal["chosen_theta"]
    assert r.split("adversarial", t).precision_on_answered >= 0.75
    # And every smaller θ on the grid fails the floor.
    smaller = [p for p in cal["curve"] if p["theta"] < t]
    assert all(p["adversarial"]["precision_on_answered"] < 0.75 for p in smaller)


def test_report_dict_carries_all_splits_and_tiers():
    r = _report(theta=0.1)
    d = r.as_dict()
    assert set(d["splits"]) == {"direct", "paraphrase", "adversarial"}
    assert d["splits"]["direct"]["tier1_p1"] == 1.0
    assert 0.0 <= d["splits"]["adversarial"]["tier2_p1"] <= 1.0
