# allerx

Desktop tool for pharmacists/physicians: check a patient's medication history before an allergy assessment. Searches HOSxP (hospital MySQL) read-only — answers "has this patient ever received this drug, and when?"

## Stack

- Tauri 2 (desktop shell) + Leptos 0.8 (WASM frontend)
- Rust workspace: `models` / `hosxp-connector` / `search-core`
- Read-only against HOSxP, enforced in layers (DB grants, read-only session, query guard)

See `AGENTS.md` (product/architecture) and `docs/DESIGN.md` (visual system) for details.
