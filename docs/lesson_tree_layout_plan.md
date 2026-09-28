# Lesson tree layout: implemented design

The downward dependency layout, collapsible units, continuous edges, canvas compaction, neighbor motion, viewport anchoring, accessibility state, touch navigation, and live-rescan handling have shipped. The bundled curriculum contains 100 lessons; the graph and assets are validated by the workspace tests.

The map treats units as course modules and lessons as their dependent classes. It should show where the player is, what is available next, and why another lesson is locked. The implementation lives in `crates/harmonicon-lessons/src/lesson_tree/`; curriculum content is described in `docs/lessons_plan.md`.

The only conditional addition is a minimap, if real usability evidence shows that navigation needs one. See `PLAN.md`. Git history records the completed delivery sequence.
