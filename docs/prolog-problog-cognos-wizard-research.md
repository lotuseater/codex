# Prolog And ProbLog In Cognos, Wizard_Erasmus, And Codex

## Scope

This note studies how local `Cognos` and `Wizard_Erasmus` use Prolog/ProbLog,
then turns that evidence into a bounded recommendation for Codex. It is
Prolog/ProbLog-focused and should be read next to the broader prior notes:

- `docs/custom-codex-wizard-cognos-research.md`
- `docs/custom-codex-wizard-cognos-plan.md`

The goal is not to port Cognos or Wizard into Codex. The useful pattern is to
let the LLM produce semantic intent and candidate facts, then let a small logic
layer perform deterministic checks and bounded evidence scoring.

## Sources Inspected

Local Cognos:

- `C:\Users\Oleh\Documents\GitHub\Cognos\docs\real_reasoners_build_and_test.md`
- `C:\Users\Oleh\Documents\GitHub\Cognos\src\logic\solver_bridge.*`
- `C:\Users\Oleh\Documents\GitHub\Cognos\src\logic\problog_bridge.*`
- `C:\Users\Oleh\Documents\GitHub\Cognos\tools\problog_bridge.py`
- `C:\Users\Oleh\Documents\GitHub\Cognos\tests\roadmap_smoke.cpp`

Local Wizard_Erasmus:

- `C:\Users\Oleh\Documents\GitHub\Wizard_Erasmus\docs\research\first_moves_logic_overlay_eval.md`
- `C:\Users\Oleh\Documents\GitHub\Wizard_Erasmus\scripts\build_first_moves_logic_overlay.py`
- `C:\Users\Oleh\Documents\GitHub\Wizard_Erasmus\src\mcp\wizard_mcp_server.py`
- `C:\Users\Oleh\Documents\GitHub\Wizard_Erasmus\docs\cognos_codex_synthesis.md`

Local Codex:

- `codex-rs/first-moves/src/predict.rs`
- `codex-rs/first-moves/src/shadow.rs`
- `codex-rs/reasoning-logic/src/lib.rs`
- `codex-rs/reasoning-logic/tests/reasoning_compare.rs`

Web sources:

- [SWI-Prolog embedding manual](https://www.swi-prolog.org/pldoc/man?section=embedded)
- [ProbLog basic modeling documentation](https://problog.readthedocs.io/en/latest/modeling_basic.html)
- [Logic-LM: Empowering Large Language Models with Symbolic Solvers for Faithful Logical Reasoning](https://arxiv.org/abs/2305.12295)
- [LINC: A Neurosymbolic Approach for Logical Reasoning by Combining Language Models with First-Order Logic Provers](https://arxiv.org/abs/2310.15164)

## Cognos Findings

Cognos uses real logic engines, not only architecture prose.
`docs/real_reasoners_build_and_test.md` states that the build/test path uses an
embedded SWI-Prolog bridge and a ProbLog bridge backed by a Python helper using
the installed `problog` package.

`PrologBridge` is deterministic. It validates structured action facts against
policy-like predicates:

- missing preconditions
- forbidden actions
- blocked actions
- confirmation requirements
- valid plans

The bridge seeds default rules for destructive actions and storage/knowledge-base
readiness. Tests in `tests/roadmap_smoke.cpp` exercise verdicts such as
`Valid`, `MissingPreconditions`, `RequiresConfirmation`, `Blocked`, and
`Forbidden`.

`ProbLogBridge` handles bounded uncertainty. It writes a temporary model, calls
`tools/problog_bridge.py`, and reads probabilities back. The tested use cases are
not broad "thinking"; they are scoped estimates such as action execution/harm
risk and visual ready/blocked probabilities.

The useful Cognos lesson for Codex is: deterministic gates should be used only
when the facts are explicit, and probabilistic inference should be used only when
the candidate space and uncertainty are bounded.

## Wizard_Erasmus Findings

Wizard has two logic lines.

The older line exposes MCP-style Prolog helpers in `wizard_mcp_server.py`, such
as fact insertion, security checks, task scheduling, and auto-analysis. That line
is useful as a historical reference but is too broad for direct Codex runtime
adoption.

The stronger current line is the `first_moves` logic overlay. It is closer to
what Codex needs:

- cached overlay data is cheap and default-friendly
- live SWI-Prolog/ProbLog is bounded and optional
- failures are fail-open
- telemetry records whether logic sources helped
- logic is advisory ranking evidence, not mandatory runtime policy

`scripts/build_first_moves_logic_overlay.py` builds Prolog/ProbLog-derived hints
for first-move ranking. `wizard_mcp_server.py` exposes `logic_mode` choices such
as `off`, `cached`, `live_bounded`, `live_relaxed`, and `auto`, plus a
`first_moves_logic_advice` path that can combine cached and bounded live
signals.

The useful Wizard lesson for Codex is: the first-moves problem is a good fit for
limited logic because the facts are already structured as prompt intent,
candidate paths, path classes, history, and prior loaded files.

## Codex Findings

Codex already has a logic foothold in `codex-rs/reasoning-logic`. That crate is
intentionally isolated and optional. It defines deterministic and probabilistic
reasoner traits, a Rust baseline, SWI-Prolog comparison support, and ProbLog
comparison support.

That crate should remain an eval/comparison surface for now. It should not be
made a dependency of normal first-turn context injection until shadow evidence
proves a runtime interface is worth the cost.

The more direct Codex insertion point is `codex-rs/first-moves`. Its current
predictor already has structured facts:

- prompt intent
- prompt/path term overlap
- explicit path and filename mentions
- already loaded paths
- baseline/source/test/docs file classes
- local hit/miss history
- project/problem memory hints
- shadow telemetry

Those are exactly the facts a limited logic/evidence layer can safely consume.

## Web Research Synthesis

SWI-Prolog's embedding model supports using Prolog as a logic service inside a
larger host application. That maps to Cognos's deterministic validation bridge:
the host owns the runtime, and Prolog checks a narrow set of predicates.

ProbLog extends Prolog-style modeling with probabilistic facts and related
constructs. That maps to bounded evidence scoring: use probabilities for
explicit uncertainty, not as a vague substitute for LLM intuition.

Logic-LM and LINC both support the same architectural conclusion: LLMs can be
paired with symbolic solvers by translating natural-language tasks into formal
or semi-formal structures, then delegating constrained reasoning to the solver.
The main risk is translation quality. If the LLM produces bad facts, the solver
can be confidently wrong. That is why Codex should use logic only where facts
come from concrete repo metadata and observable user prompt structure.

## Recommendation

Prolog/ProbLog is useful for Codex, but only in a limited form.

Borrow immediately for `first_moves`:

- deterministic gates over structured path facts
- bounded probabilistic evidence scoring for candidate ranking
- fail-open behavior
- shadow telemetry comparing current and logic-enhanced ranking
- no external runtime requirement

Do not borrow yet:

- always-on SWI-Prolog or ProbLog subprocesses in normal turns
- broad planner/reviewer blackboards
- runtime policy enforcement
- `codex-core` expansion
- vague probabilistic "intuition" layers

The practical direction is Rust-native logic/evidence in `codex-rs/first-moves`,
with SWI-Prolog/ProbLog kept as optional comparison tools in
`codex-rs/reasoning-logic`.
