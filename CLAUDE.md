@AGENTS.md

## Claude Code

Run the checks under "Definition of done" in AGENTS.md before you say a change is finished. If `cargo` is not installed on the machine, say so in your hand-off instead of skipping the checks quietly, and let CI run them on a branch.

## Subagents

Use subagents. They keep this conversation small and let independent work run in parallel, so reach for them by default instead of doing everything in one thread: wide searches across many files, independent changes in separate modules, tests written alongside the code they cover, long verification runs, and a fresh-eyes review of a diff before it is merged (for example writing integration tests for separate commands in parallel, or a standalone review of a change to the crypto code). Do a small, strictly sequential task yourself.

Brief each agent like a colleague who has not seen this conversation: the goal, the files or docs to read, what is already ruled out, and what to report back. Set `model` explicitly on every call, because leaving it out silently inherits the session's model.

| Tier | Use for |
| --- | --- |
| `haiku` | Mechanical bulk work: renames, boilerplate, format conversion, log triage, simple lookups |
| `sonnet` | The default: well-specified implementation, test writing and routine migrations with clear acceptance criteria |
| `opus` | Genuinely tricky work: concurrency, subtle algorithms, security-sensitive changes, adversarial review or verification, gnarly debugging |

Pick the cheapest tier that can do the job and escalate on failure. Never use `fable` for a subagent unless the user explicitly asks for it.
