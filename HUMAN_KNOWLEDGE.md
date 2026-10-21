# HUMAN_KNOWLEDGE.md

What the human decides and keeps track of, outside the code. An agent can read this file and
propose changes to it, but the boxes are ticked by the human.

## SOCIAL PLANNING

### Rules

- [ ] **Threads gets one post every week.** The weekly post is written by hand; nothing in the
  repo posts on a schedule.
- [x] **Every release gets a Threads post.** `release.yml` dispatches `threads.yml`, which runs
  `nix run ./tools#social -- release <tag>` ([ADR 0001](docs/adr/0001-automatic-posts-to-threads.md)).
- [x] **Every Instagram post is mirrored on Threads.** Each `docs/social/instagram_*/README.md`
  carries `status: DESIGN | REVIEW | POSTED`. When a push to `main` turns one to `POSTED` (after
  the post is live on Instagram), `threads.yml` posts the same slides, caption and alt texts.
  CI (`social posts`) fails when a post has no valid status.

### Networks

Box ticked: the network is in use. **Possible now** answers one question: can picasso as it is
today (a public repo, GitHub Actions with repository secrets, `tools/social` (Rust), personal
accounts, no registered legal organization) post there automatically? Checked on 2026-10-12;
"not checked" means nobody has read the official documentation yet.

| In use | Network | Possible now | Why |
|---|---|---|---|
| [x] | Threads | **Yes, done** | Threads API with the Threads Tester role, no App Review; token 60 days, refreshable (`just threads-refresh`). [Docs](https://developers.facebook.com/docs/threads/posts) |
| [x] | Instagram (personal) | **No, by hand** | The API publishes only for Business or Creator accounts. Posts are made by hand and are the source for Threads. Not checked again today. |
| [ ] | LinkedIn, personal profile | **Yes, paused** | Share on LinkedIn (`w_member_social`) is self-serve, but the token lasts 60 days with no refresh token, so it needs a new login every 60 days. Taken out of the flow on 2026-10-12. [Docs](https://learn.microsoft.com/en-us/linkedin/consumer/integrations/self-serve/share-on-linkedin) |
| [ ] | LinkedIn, company page | **No** | `w_organization_social` needs the Community Management API, only for registered legal organizations. [Docs](https://learn.microsoft.com/en-us/linkedin/marketing/community-management-app-review) |
| [ ] | Bluesky | **Yes** | Open AT Protocol API with an app password; images are uploaded as blobs. Not implemented. Not checked today. |
| [ ] | Mastodon | **Yes** | Open REST API with an access token from the account's settings. Not implemented. Not checked today. |
| [ ] | X | **Yes, paid** | Pay-per-use API since February 2026; each post costs money and the free tier is closed to new developers (third-party sources, not the official pricing page). |
| [ ] | Medium | **No** | The public API is not supported and new integration tokens are not issued (third-party sources, not Medium's own docs). |
| [ ] | Substack | **No** | No documented public API to publish; tools that exist use a session cookie. Not checked against Substack's docs. |
| [ ] | DEV (dev.to) | **Yes** | Forem API with a personal API key creates articles; fits the whitepaper or a study. Not checked today. |
| [ ] | Hashnode | **Yes** | GraphQL API with a personal token publishes posts. Not checked today. |
| [ ] | Facebook page | **No** | Page posts need a Meta app with App Review for page permissions. Not checked today. |
| [ ] | NixOS Discourse | **Yes, but by hand** | Discourse has an API key per user, but an automatic post to a community forum is not welcome; post by hand. |
| [ ] | Hacker News, Lobsters, Reddit | **No** | No posting API (HN, Lobsters) or rules against self-promotion bots (Reddit); share by hand. |
| [ ] | YouTube | **No** | Needs a video; nothing in the repo makes one. |
