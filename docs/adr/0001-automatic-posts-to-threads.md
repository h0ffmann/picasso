# 0001: Automatic posts to Threads

- **Status**: Proposed
- **Date**: 2026-10-12
- **Deciders**: Hoffmann

## Context

picasso announces its work by hand. The first post, [instagram_001](../social/instagram_001/), was
designed in the repo and posted by hand on Hoffmann's personal Instagram
(<https://www.instagram.com/p/DeZal3lkfRh/>). A release (`just release`, `release.yml`) makes a
GitHub release and a Zenodo DOI, but nothing tells people about it. We want two things on
**Threads**, with no manual step after the human decision:

- every Instagram post goes to Threads with the same content;
- every release gets a Threads post.

The publishing plan for all networks, with what is possible today, is in
[HUMAN_KNOWLEDGE.md](../../HUMAN_KNOWLEDGE.md#social-planning). The facts below come from the
official documentation, read on 2026-10-12.

### Threads (Meta)

- Publishing is two calls: `POST /{threads-user-id}/threads` makes a media container (`TEXT`,
  `IMAGE`, `VIDEO` or `CAROUSEL`), then `POST /{threads-user-id}/threads_publish` publishes it.
  Wait until the container status is `FINISHED` before the publish call.
  ([Posts](https://developers.facebook.com/docs/threads/posts))
- Limits: 500 characters of text (an emoji counts by its UTF-8 length), 2 to 20 items in a
  carousel, 250 posts for each profile in 24 hours, at most 5 distinct links, and a link preview
  only on a text post. An image must be at a public URL when the container is made; there is no
  upload. Images are JPEG or PNG, at most 8 MB, 320 to 1440 px wide; `alt_text` takes up to 1000
  characters. ([Posts](https://developers.facebook.com/docs/threads/posts),
  [Overview](https://developers.facebook.com/docs/threads/overview),
  [Publishing reference](https://developers.facebook.com/docs/threads/reference/publishing))
- Permissions: `threads_basic` and `threads_content_publish`, on a Meta app with the Threads use
  case. A user with a role on the app (a **Threads Tester**) can grant them without App Review.
  Only Hoffmann's own profile posts, so App Review is not necessary.
  ([Get started](https://developers.facebook.com/docs/threads/get-started))
- Tokens: a short-lived token lasts 1 hour; exchange it for a long-lived token that lasts
  **60 days**. Refresh it (`grant_type=th_refresh_token`) when it is at least 24 hours old and not
  expired; each refresh gives 60 more days. An expired token cannot be refreshed.
  ([Long-lived tokens](https://developers.facebook.com/docs/threads/get-started/long-lived-tokens))

### LinkedIn (deferred)

LinkedIn was in the first version of this ADR. On 2026-10-12 Hoffmann took it out of the flow for
now. The research stays here for a later ADR.

#### Personal profile

- The **Share on LinkedIn** product is self-serve: add it to the app in the Developer Portal and
  it gives `w_member_social`, which posts on behalf of the member who authorized the app.
  ([Share on LinkedIn](https://learn.microsoft.com/en-us/linkedin/consumer/integrations/self-serve/share-on-linkedin))
- The current endpoint is `POST https://api.linkedin.com/rest/posts` with the headers
  `Linkedin-Version: YYYYMM` and `X-Restli-Protocol-Version: 2.0.0`; `author` is
  `urn:li:person:{id}`. Images are uploaded first through the Images API; an organic post with
  several images uses MultiImage (organic carousels are not supported).
  ([Posts API](https://learn.microsoft.com/en-us/linkedin/marketing/community-management/shares/posts-api))
- Each `Linkedin-Version` is retired about one year after its release. For example, 202510 is
  retired on 2026-10-15 (notice on the same page). The pinned version must be bumped.
- Tokens: an access token lasts **60 days**. Programmatic refresh tokens (365 days) are given
  only to approved Marketing Developer Platform partners, so this app gets none: **a person
  must authorize again every 60 days**.
  ([Refresh tokens](https://learn.microsoft.com/en-us/linkedin/shared/authentication/programmatic-refresh-tokens))
- Rate limit: 150 requests for each member per day (Share on LinkedIn page).

#### Company page

Posting to a company page **is possible through the API, but not for this project now**:

- It needs `w_organization_social`, and the member who authorizes must be an ADMINISTRATOR,
  CONTENT_ADMIN or DIRECT_SPONSORED_CONTENT_POSTER of the page.
  ([Posts API](https://learn.microsoft.com/en-us/linkedin/marketing/community-management/shares/posts-api))
- That permission comes only with the **Community Management API**, a vetted product. LinkedIn
  says: "our Community Management APIs are only available to registered legal organizations for
  commercial use cases only". The request needs a verified business email (personal addresses
  fail), the organization's legal name, registered address, website and privacy policy, and a
  super admin of the page must verify the app. The product must be the only product on its app,
  and a rejected app cannot apply again.
  ([App review](https://learn.microsoft.com/en-us/linkedin/marketing/community-management-app-review),
  [Overview](https://learn.microsoft.com/en-us/linkedin/marketing/community-management/community-management-overview))
- The Development tier allows 500 calls for each app and 100 for each member in 24 hours. The
  Standard tier needs a second review with a screencast of the OAuth flow and of posting, and an
  app must apply for it within 12 months or lose access.

picasso is a personal project with no registered legal organization, so a company page (for
picasso, marola-dev or ouroboric-dev) cannot post automatically today. If one of those becomes a
legal organization, a new ADR can add it.

## Decision

1. **Target**: Hoffmann's Threads profile, through a Meta app where Hoffmann is a Threads Tester.
   No other network is automated now.
2. **Instagram posts carry a status.** Each `docs/social/instagram_*/README.md` starts with front
   matter:

   ```yaml
   ---
   status: POSTED                                   # DESIGN | REVIEW | POSTED
   instagram: https://www.instagram.com/p/<id>/     # required when POSTED
   caption: pt-BR                                   # the "## Caption (<lang>)" block that was posted
   ---
   ```

   CI (`social posts` in `ci.yml`) runs `nix run ./tools#social -- check` and fails when a post has no
   valid status, a POSTED post has no Instagram URL, or the caption is longer than Threads allows.
3. **Instagram to Threads.** `threads.yml` runs on a push to `main` that changes a post README.
   It posts every post whose status became `POSTED` in that push: the caption block named by
   `caption`, the `slide-N.png` files in order as a carousel (at their raw GitHub URL at the pushed
   commit) and the "## Alt text" items as alt texts. The human gate is the PR that sets `POSTED`,
   merged after the post is live on Instagram, so the content on Threads is content that is already
   public. `workflow_dispatch` with `post` (a directory, or `latest`) posts one again by hand.
4. **Releases to Threads.** After it makes the GitHub release, `release.yml` dispatches
   `threads.yml` with `release=<tag>` (a release made with `GITHUB_TOKEN` starts no other
   workflow). `nix run ./tools#social -- release <tag>` posts a short pt-BR text with the release notes
   link (as the link preview), the concept DOI and the site.
5. **Code**: `tools/social`, a Rust crate (it was `scripts/social.py`, standard library only, until picasso became Nix and Rust only), with
   `--self-test` (fake HTTP), `--dry-run` for `post` and `release`, and `check`. The root
   `justfile` gets `social` and `threads-refresh`, in just 1.21 syntax (no recipe attributes).
6. **Secrets**: `THREADS_USER_ID` and `THREADS_ACCESS_TOKEN`, repository secrets, never in a file.
   `just threads-refresh` renews the token and pipes it into `gh secret set`, so it is never
   printed. Run it at least once every 60 days.
7. **Weekly post**: Threads should get a post every week. That is a human plan in
   HUMAN_KNOWLEDGE.md, not a schedule in the repo.

## Alternatives

- **A third-party scheduler (Buffer, Hootsuite, Zapier, Make)**: hides the token work, but adds an
  account, a paid plan for more channels and a dependency outside the repo. Rejected for now.
- **A scheduled workflow that refreshes the token**: removes the 60-day chore, but needs a PAT
  that can write repository secrets. Rejected for now; it can come in a later ADR.
- **A `social` environment with a required reviewer**: a second approval for each post. The PR
  that sets `POSTED` (and the release tag) is already a human decision, so this adds a click and
  nothing more.
- **Post on Threads first and Instagram after**: the repo would have to post to Instagram too,
  which needs a Business or Creator account. Rejected.

## Consequences

- Merging this ADR's PR posts instagram_001 to Threads once, because its README turns `POSTED`
  in the merge, if the two secrets are set by then. Without them the run fails with a clear
  message, and `workflow_dispatch` with `post=docs/social/instagram_001` posts it later.
- A re-run of the same push posts again; do not re-run a green `threads` run.
- The token must be renewed every 60 days with `just threads-refresh`, or a new login is needed.
- Slides must be committed and the repo public: Threads fetches them from
  `raw.githubusercontent.com`.
- The release text is fixed in `tools/social/src/post.rs`; change it there.
