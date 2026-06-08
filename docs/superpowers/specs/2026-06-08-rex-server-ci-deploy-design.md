# CI deploy for rex server (GitHub Actions → Lightsail) — design

**Date:** 2026-06-08
**Status:** Approved, ready for implementation

## Problem

Deploying the rex server currently means running `ren-infra/aws/rex/deploy.sh` from a
dev laptop, which builds the binary **on the 2 GB Lightsail box** and rsyncs `rex.db` +
PDFs from local. For a code-only change (like the related-files feature), the data sync
and on-box build are unnecessary overhead, and the deploy is tied to one laptop. We want
a one-click GitHub Action that builds the binary in CI and ships it to the box.

## Scope

**Code-only deploy of the rex server binary.** CI does not have `rex.db` or the PDF
corpus (they live on the dev box), so data deploys stay on the manual
`deploy.sh --rsync-pdfs` path. This feature needs no DB/PDF change.

Out of scope (YAGNI): client deploy (Vercel auto-deploys on push), DB/PDF sync,
rollback automation (the runbook's `git checkout + rebuild` covers it), auto-trigger.

## Box facts (from `ren-infra`)

- Lightsail `rex-prod`, **Ubuntu 22.04**, 2 vCPU / 2 GB RAM, ports 22/80/443.
- Binary: `/usr/local/bin/rex`. Service: `rex.service`. SSH user: `ubuntu` (non-interactive sudo).
- Backend-only box; the Next client builds on Vercel.
- Build is pure Rust: `rex-llamacpp` defaults to the `stub` feature (no C++), and there
  are no `build.rs`/`-sys` crates — so CI needs only the stable Rust toolchain.

## Architecture

One component: `.github/workflows/deploy-rex-server.yml` in the `ren-rex` repo.

**Trigger:** `workflow_dispatch` only (manual). `concurrency: deploy-rex` (no overlap,
no cancel-in-progress).

**Runner:** `ubuntu-22.04` — pinned to match the box's glibc 2.35 so the dynamically
linked binary is ABI-compatible. (Not `ubuntu-latest`, which is 24.04.)

**Steps:**
1. `actions/checkout@v4` — deploys whichever branch is chosen in the Run-workflow UI.
2. `dtolnay/rust-toolchain@stable`.
3. `Swatinem/rust-cache@v2` with `workspaces: server`.
4. `cd server && cargo build --release --bin rex`.
5. Configure SSH: write `secrets.REX_DEPLOY_SSH_KEY` to `~/.ssh/deploy_key` (via env var,
   not inline, so it never lands in a command line), `chmod 600`, and pin the host key
   with `ssh-keyscan -H "$REX_DEPLOY_HOST" >> ~/.ssh/known_hosts`.
6. Ship + swap (mirrors `deploy.sh`):
   ```
   scp -i ~/.ssh/deploy_key server/target/release/rex ubuntu@$HOST:/tmp/rex-new
   ssh -i ~/.ssh/deploy_key ubuntu@$HOST \
     'sudo install -m 0755 /tmp/rex-new /usr/local/bin/rex && rm -f /tmp/rex-new && sudo systemctl restart rex.service'
   ```
7. Smoke test (fails the job on any miss):
   - `curl -sf http://127.0.0.1:8080/v1/health` succeeds.
   - `curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:8080/v1/documents/not-a-uuid/related-files` returns **400**.
     This proves the *new* binary is live: the new route parses the id and rejects a
     non-UUID with 400, whereas the old binary (no such route) would return 404. An
     unknown-path probe would not discriminate (both binaries 404 unknown paths).

**Secrets (added once, not committed):**
- `REX_DEPLOY_SSH_KEY` — dedicated ed25519 private key for CI.
- `REX_DEPLOY_HOST` — `54.255.200.145`.
- SSH user `ubuntu` hardcoded in the workflow.

## One-time setup (operator runs; not in the workflow)

1. Generate a dedicated keypair: `ssh-keygen -t ed25519 -f ~/.ssh/rex-ci-deploy -N '' -C 'rex-ci-deploy'`.
2. Authorize the public key on the box (using existing Lightsail key):
   `ssh -i <lightsail-key> ubuntu@54.255.200.145 "echo '<pubkey>' >> ~/.ssh/authorized_keys"`.
3. Set secrets: `gh secret set REX_DEPLOY_SSH_KEY < ~/.ssh/rex-ci-deploy` and
   `gh secret set REX_DEPLOY_HOST -b 54.255.200.145` (repo `ren-education/ren-rex`).

## Docs

Add a "CI code deploy" note to `ren-infra/runbooks/rex-deploy.md`: CI = code-only binary
swap (manual workflow); `deploy.sh` = code + data (DB/PDF rsync from dev). Use CI for
code changes, `deploy.sh --rsync-pdfs` after ingesting new content.

## Verification

- YAML parses / workflow is listed by `gh workflow list` after push.
- A live run builds, ships, restarts, and both smoke checks pass (health 200,
  related-files probe 400).
- Manual confirmation: `https://rex.reneducation.com` shows the related-files links.
