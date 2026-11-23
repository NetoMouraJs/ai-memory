# Fork customizations

This fork (`NetoMouraJs/ai-memory`) tracks `akitaonrails/ai-memory` and adds
the customizations below. Upstream's `CHANGELOG.md` stays untouched so every
upstream merge applies cleanly; fork changes are listed here instead.

## Branch model

- `main` is upstream plus every customization, and every installed build comes
  from it.
- Upstream updates arrive by merge, never rebase: `git fetch upstream`, then
  `git merge upstream/main` (or a release tag) on `main`, resolving conflicts
  in the merge commit. Published history is never rewritten.
- Each customization is developed on a `custom/<name>` branch cut from `main`
  and merged back with `git merge --no-ff`.
- Nothing goes back upstream: the `upstream` remote has no push URL.

## Customizations

### `custom/ai-dir-marker`

After the normal walk up from the working directory, the marker is also looked
for as `.ai/.ai-memory.toml` at the checkout root and, in a linked worktree, at
the main checkout found through the common git directory. Resolution order, at
the checkout-root step: nested marker > root marker > the checkout's own
`.ai/` marker > the main checkout's `.ai/` marker. An `.ai/` marker's
`[capture] ignore_paths` are anchored at the current checkout root. Only the
native `ai-memory hook` honours it; the shell and PowerShell bundles and the
generated TypeScript plugins do not.

### `custom/hook-inject-env`

Environment an orchestrator sets on its workers, read by the native
`ai-memory hook` only (see "Orchestrator workers" in `docs/usage.md` and row
4y of `docs/security-boundaries.md`):

- `AI_MEMORY_HOOK_INJECT` = `all` (default) | `no-handoff` | `none`.
  `no-handoff` keeps recovery, anchors, recaps, briefs and the profile digest,
  withholds the operator's handoff and inbox notice (`handoff=0&inbox=0`), and
  never closes the interrupted session a recovery names. `none` sends no
  context request. An unknown value counts as `none`. Capture never changes.
- `AI_MEMORY_SESSION_ROLE` = `automation` | `builder` | `reviewer` | `qa`,
  stored as the session origin; every origin counts as automation for
  `[profile] harvest_prompts = "interactive"`.
- `AI_MEMORY_SESSION_LABEL`: free text, sent as the session's display name in
  place of the harness's own.

The server accepts `inbox=0` on `GET /handoff` and logs each worker start
(`orchestrated session start`). `ai-memory capabilities --json` reports the
supported values, since the fork keeps upstream's version number. No schema
migration is added, so upstream migrations keep applying in order.

### `custom/capabilities-build`

`ai-memory capabilities` also reports `"build": "ownercode"` and
`"repository": "https://github.com/NetoMouraJs/ai-memory"`, so a launcher can
tell this build from upstream's without inferring it from the worker fields.

### `custom/service-control`

`ai-memory service start|stop|status [--json]` controls the Windows logon host
(`service-host`) without waiting for the next logon or killing it by pid:

- `stop` is the host's cooperative stop (`service-host.stop`, `taskkill` only
  after 10 s without an answer); the `Run` entry stays.
- `start` leaves a running host alone, refuses when another server already
  answers on the configured URL, and otherwise starts the host and waits up
  to 30 s for its server to answer.
- `status --json` prints `running`, `supervisor_pid`, `server_pid`,
  `responding` and `server_url`.

When the caller's job forbids breakaway (the terminal of an IDE or an agent
app), `service-host` now starts the supervisor through WMI
(`Win32_Process.Create`, window hidden), which places it outside that job.
Before, the supervisor stayed in the job and the app closing killed the
server with no log line. It still falls back to starting inside the job,
with a warning, when WMI fails. `capabilities` reports `"service_control"`
(true on Windows).

### `custom/idle-end-interval`

The idle-session auto-end tick (`[sessions] idle_end_after_hours`) scans once
an hour instead of every 15 minutes (`IDLE_END_INTERVAL_SECS`). A session
idle past the cutoff now ends up to an hour after it, not up to 15 minutes;
the cutoff itself is still the config value.
