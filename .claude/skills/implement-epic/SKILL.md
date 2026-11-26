---
name: implement-epic
description: Implement a whole Bears epic unattended, from its planned tasks to a pull request with green CI, by orchestrating one task-implementer subagent per task and a task-reviewer for each.
argument-hint: <epic-id>
disable-model-invocation: true
---

Implement the Bears epic `$ARGUMENTS` from start to a pull request with green CI, without
the user watching. They read one thing at the end: the pull request and your final report.

You are the orchestrator. Each task goes to a `task-implementer` subagent and each diff to
a `task-reviewer` subagent, so that your context lasts for the whole epic; you read no
full diffs. What only you can do is the part between the tasks: choosing the order,
telling each agent what the others built, deciding on review findings, integrating the
branches, and judging whether the epic as a whole is done. The one exception is visual
design (see "The loop").

`CLAUDE.md` is the rulebook, including its exception that lets each task commit once its
review passes. Use the `bea` CLI throughout, like the agents, so that everyone sees the
same store.

## Before starting

Stop and tell the user, instead of working around it, if any of these fails:

- `bea show <epic> --json` is an epic with planned sub-tasks (`bea list --epic <epic> -a --json`).
- Every epic or task in the epic's own `depends_on` is done. `bea ready --epic` does not
  check this, so check it yourself.
- The working tree is clean, and `main` is up to date with `origin/main`.
- `build/` holds `styles.css`, `codemirror.js` and `mermaid.js`. If not, run `npm ci` and
  `npm run build` once yourself.
- If the epic touches the backup server (`backend/`, `granit-api/`), a Postgres answers at
  the `DATABASE_URL` of `backend/README.md` ("Development"). Without one the server's
  database tests and `cargo sqlx prepare` cannot run.

Then read `CLAUDE.md`, the epic's body, every task's body and dependencies
(`bea show <id> --json` for each; the list output leaves the dependencies out), and the
docs pages the epic names. You need to know the epic well enough to tell a correct seam
from a wrong one. Do not read the code the tasks will change.

Create the epic's branch from `main`: `<type>/<short-slug>` (`feat/presentations`,
`fix/tree-background-clicks`). If the branch already exists, this is a resumed run: check
it out, see what is committed, `bea release` any task left in progress with no commit, and
continue with the loop.

## The loop

Keep your own list of the task ids handed out and not yet integrated: a worktree task's
`bea start` lands only in its worktree's copy of `.bears/`, so the main checkout still
shows it as ready. Repeat while `bea ready --epic <epic> --json` holds a task that is
neither tagged `manual` nor on your list, or an agent is still running.

A task tagged `manual` is the user's to do: never give it to an implementer. Tasks of this
epic can depend on tasks of other epics, and those you do not implement either. So the
loop can end with tasks still open: blocked by a manual task, by a task outside the epic,
or by a task you released. That is a partial run; see "Finishing the epic".

A task that is mainly visual design (a theme, a new look for a panel: layout and styling
choices rather than wiring) you do yourself, in the main checkout and with nothing else
running there, so that the epic's look stays under one hand. Follow the implementer's
steps for it (`bea start`, checks, `bea done`, one commit) and still have it reviewed.

### 1. Choose the batch

Take the ready tasks. Run them one at a time unless running some together is safe:

- At most one task at a time that changes a shared seam: the IPC surface (`granit-types`,
  the `generate_handler!` list in `src-tauri/src/lib.rs`, `src/app/ipc.rs`), any
  `Cargo.toml` or `Cargo.lock`, `package.json`, the JS bundles in `js/`, or the backup
  server's migrations and `.sqlx/`.
- Two tasks can run beside each other when their bodies make it clear that they touch
  different modules (say, an agent tool in `src-tauri/src/agent/tools/` and a docs page, or
  two separate frontend components). If you cannot tell, run them in sequence.
- Two agents at once by default, three at most: every checkout compiles the workspace,
  which is heavy (`fastembed`, `ort`, Tauri).

A single task, and the seam-changing task of a parallel batch, works in the main checkout
on the epic's branch. Every other task of a batch gets its own worktree: spawn it with
`isolation: "worktree"`. The project settings base such a worktree on the current HEAD, so
the main checkout must be clean and committed when you spawn it, and `.worktreeinclude`
copies `build/` into it.

### 2. Spawn the implementers

One `task-implementer` per task, all of a batch in the same message. Its prompt holds:

- the task id and the epic id (it reads the bodies itself; do not paste them);
- where it works: the main checkout on the epic's branch, or its worktree; for a worktree,
  the main checkout's absolute path, so that it can share the main `target/` directory;
- the seams it builds on, from earlier implementers' reports: the commands, types,
  `AppCtx` signals, components, tools or modules that exist now and where they are;
- decisions already made in this epic that bind it;
- for a batch: which other tasks run beside it and what they own, so that it keeps out.

### 3. When an implementer reports

Check the report against the repository, briefly: there is exactly one new commit
(`git -C <dir> log --oneline <base>..HEAD`), the tree is clean, and
`git -C <dir> diff --stat <base>..HEAD` touches what the task is about. Do not read the
diff.

If it stopped without finishing, decide: answer it with SendMessage if you can resolve what
blocked it from the epic, `CLAUDE.md` or the docs; otherwise `bea release <id>`, note it
for the final report, and go on with the tasks that do not depend on it.

Otherwise spawn a `task-reviewer` with the task id, the directory, the commit range and,
for a worktree, the main checkout's `target/` path, and nothing else. It must not see the
implementer's report: a reviewer handed the author's account tends to confirm it.

### 4. When a reviewer reports

Decide on every finding yourself; the reviewer advises.

- Blocking and should-fix findings you agree with go back to the same implementer with
  SendMessage, so that it fixes them with its context intact. Send the findings as the
  reviewer wrote them, with the `file:line` and the failing case. The implementer amends
  its task commit; it does not add a fix commit.
- A finding you judge wrong, or outside the task, you drop, with the reason kept for the
  final report. A finding that is real but belongs to another task becomes a Bears task
  (`bea create … --parent <epic>`, with `bea dep add` where it blocks something).
- After fixes, send the reviewer the new range with SendMessage for a second look at just
  those findings. Two rounds at most: what is still open after the second becomes a Bears
  task under the epic, and you move on.

Do not start another task in the main checkout while the one there is still under review:
its commit must stay at HEAD so that it can be amended.

### 5. Integrate

Integrate only while no agent is working in the main checkout. A task done in the main
checkout is already on the epic's branch. For each worktree task, in dependency order:

1. `git -C <worktree> rebase <epic-branch>`. A conflict only in `Cargo.lock`: take the
   epic branch's side (`git -C <worktree> checkout --ours Cargo.lock`; during a rebase
   "ours" is the branch being rebased onto), run
   `cargo update -w --manifest-path <worktree>/Cargo.toml`, `git -C <worktree> add Cargo.lock`
   and `git -C <worktree> -c core.editor=true rebase --continue`. A conflict only in
   `backend/.sqlx/`: the same, then regenerate it with `cargo sqlx prepare`. Any other
   conflict: `git -C <worktree> rebase --abort` and send it to the task's implementer,
   which knows both the code and its worktree.
2. `git merge --ff-only <task-branch>` in the main checkout. Never a merge commit.
3. `git worktree remove <worktree>` and `git branch -d <task-branch>`, and take the task off
   your list.

After a batch with a worktree task is integrated, run the clippy and test commands of
`CLAUDE.md` for the crates the batch touched, on the epic's branch. Send the output to a
file in your scratchpad and read only the end of it. A failure goes to the implementer
whose task it concerns.

Discovered work from the implementers' reports becomes Bears tasks now: under the epic if
the epic is not done without it, otherwise on its own with the right priority. Commit new
and changed `.bears/` files that no implementer committed
(`chore: track work found in the <name> epic (<epic>)`).

## Finishing the epic

When the loop has ended, with every task done or with the rest blocked (a partial run):

1. Run every check of `CLAUDE.md` for every part the epic touched, on the epic's branch,
   output to files as above. If a docs page changed, build the site from `docs/` into the
   scratchpad (`aphid build -o <scratchpad>/site`; a bare `aphid build` writes into
   `docs/dist/`). Fix failures through the implementer concerned.
2. Spawn a `task-reviewer` for an epic review of `main..HEAD`. Handle its findings as in
   step 4, sending each to the implementer of the task it concerns. A fix now lands as a
   `git commit --fixup=<task-commit>`; once all are in, fold them with
   `git rebase --autosquash main`, so that each task stays one commit.
3. Read the seams yourself, and only those: the diff of `granit-types/`, the
   `generate_handler!` list, `src/app/ipc.rs`, `src/app/context.rs`, any new migration, and
   the docs pages. Check them against the epic's body and its acceptance criteria.
4. Bears closes the epic by itself when its last task is done, and the implementer of that
   task commits the change. If the epic is still open with every task done, `bea done
   <epic>` and commit it (`chore: complete the <name> epic (<epic>)`). In a partial run the
   epic stays open; the pull request's title and description say that it is a part, and
   list each open task with what blocks it.
5. Rebase on `origin/main` if it moved, push, and open the pull request with
   `gh pr create`. The title names the epic in the user's terms, like the earlier ones
   ("Presentations: markdown slides with per-cave CSS templates in a separate window").
   The description, written for a reader who has not followed the run, covers:
   - what the epic delivers, in the user's terms;
   - the tasks with their ids;
   - **decisions made**: every place where an implementer or you chose between readings,
     with the reason, so the user can overrule one cheaply;
   - findings you dropped and why, and tasks you created;
   - **verification**: what the checks covered, and what still needs a manual try in the
     running app (any UI change does: nobody launched it);
   - anything not done.
6. Watch CI (`gh pr checks <number> --watch`). A failing job is fixed through the
   implementer concerned (as a fixup folded into its commit, then
   `git push --force-with-lease`), until every check is green.

Do not merge the pull request, never bump the version, and never create a release tag.

## Deciding without the user

The user is not there, so do not stop to ask what you can decide. Where a task is open to
two readings, take the one that fits `CLAUDE.md` (backend-owned state, build only what is
needed now) and the existing docs, and record it under "decisions made". Prefer the
reading that is easier to change later.

Stop and report, with the epic's branch left as it is, only when going on would mean one
of these:

- a change that breaks existing caves (their `.granit/config.yml`, note files or
  frontmatter) without a migration path;
- a breaking change to the backup server's API, or an edit to a merged migration;
- deleting or rewriting work outside the epic;
- a task that contradicts `CLAUDE.md` in a way that changes the product;
- the same task failing after two implementers have tried it, with nothing else ready.

## The final report

A few lines: the pull request's link and whether CI is green; the tasks done and any
released; the decisions the user should look at first; what to try by hand in the app; the
Bears tasks you created. If the run stopped early, lead with what stopped it and what the
user needs to decide.
