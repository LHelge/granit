---
applyTo: "src/**"
---

# Leptos Frontend

Instructions for `src/`.

## Repo-Specific Rules

- The frontend is a thin view layer. Keep business logic, cave logic, markdown logic, and derived data in the backend when possible.
- Use the existing IPC layer in `src/app/ipc.rs` instead of inventing one-off invoke patterns.
- Use Leptos 0.8 signals and `spawn_local` in the established style already used in the repo.
- Prefer small, focused components that match the current app structure (`agent`, `editor`, `explorer`, `settings`, `components`).

## Styling

- Use Tailwind CSS 4 and DaisyUI 5 classes directly in `view!` macros.
- Prefer DaisyUI component classes over hand-rolled equivalents.
- Preserve the existing visual language unless the task explicitly asks for a redesign.

## Practical Gotchas

- In `view!`, closures usually need `move ||` so signals are captured correctly.
- Keep frontend state minimal. If you find yourself rebuilding backend state in Leptos, move that logic to a command.
