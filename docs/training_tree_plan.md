# Training tree: remaining rollout decisions

The technique graph, validators, skill-tree layout, lesson and training records, results routing, per-lesson and per-track mastery, warm-up review queue, and practice streak have shipped. The bundled curriculum contains 100 lessons. Curriculum content is described in `docs/lessons_plan.md`; tree layout is documented in `docs/lesson_tree_layout_plan.md`.

## Generated trainings

Five training tiers form an optional ladder attached to a lesson. They do not gate lesson progression: a passed lesson unlocks its successors, while trainings contribute to mastery and scheduled review. Records stay separate from lesson completions in `PlayerProfile::trainings`.

The generator and `bend` track specs (`first-bend`, `deep-bends`, `high-blow-bends`) exist. Play the generated drills with a harp and decide whether they teach the intended technique before rolling them out to other tracks. A generator passing schema and chart tests is not a musical acceptance test.

## Curriculum decisions

The `hand` track contains one lesson. Decide whether to fold it into `tone` or leave the gap visible for future lessons. Revisit other thin tracks only on actual curriculum evidence. `PLAN.md` tracks these open decisions.

The tree scrolls vertically and places improvisation lessons in the track they improvise over. The compact layout remains available; it is not an open replacement decision.
