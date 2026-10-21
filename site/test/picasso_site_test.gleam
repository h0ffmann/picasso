import gleam/int
import gleam/list
import gleam/option.{None, Some}
import gleam/order
import gleam/string
import gleeunit
import lustre/element
import lustre/element/html
import picasso_site/app
import picasso_site/data
import picasso_site/family
import picasso_site/format
import picasso_site/markdown
import picasso_site/page
import picasso_site/readiness
import picasso_site/story

pub fn main() -> Nil {
  gleeunit.main()
}

fn prerendered() -> String {
  app.initial() |> app.view |> page.document |> element.to_document_string
}

// The hard rule (AGENTS.md), checked again on the built page by site-check.
pub fn logos_and_credit_are_on_the_page_test() {
  let html = prerendered()
  assert string.contains(html, family.marola_logo)
  assert string.contains(html, family.ouroboric_logo)
  assert string.contains(html, family.ouroboric_logo_dark)
  assert string.contains(html, "https://github.com/gvolpe/nix-config")
  assert string.contains(html, "href=\"plan/hosting-plan.pdf\"")
}

pub fn favicon_is_linked_test() {
  let html = prerendered()
  assert string.contains(html, "href=\"icons/favicon.svg\"")
  assert string.contains(html, "href=\"icons/favicon.ico\"")
  assert string.contains(html, "href=\"icons/apple-touch-icon.png\"")
}

const assessment =
  "# Readiness, 2026-10-01

**Bottom line: three of five met.**

| Id | Question | Status | Evidence | To close |
|---|---|---|---|---|
| OP-10 | Is it **live**? | Met | [deploy](../ops.md) | — |
| OP-9 | Has a `CI` gate? | Missing | none | add one |
| GO-1 | Licence? | Partial | MIT only | add CC BY |
"

pub fn readiness_parses_the_table_test() {
  let r = readiness.parse(assessment)
  assert r.title == "Readiness, 2026-10-01"
  assert r.bottom == "three of five met."
  assert list.length(r.items) == 3
  assert readiness.count(r.items, readiness.Met) == 1
  let assert [first, ..] = r.items
  assert first.to_close == ""
}

pub fn readiness_sorts_open_work_first_and_ids_numerically_test() {
  let ids =
    readiness.parse(assessment).items
    |> readiness.sorted
    |> list.map(fn(i) { i.id })
  assert ids == ["OP-9", "GO-1", "OP-10"]
  assert readiness.compare_ids("OP-9", "OP-10") == order.Lt
}

pub fn markdown_renders_links_code_and_bold_test() {
  let html =
    markdown.inline(
      "See [the `plan`](../plan.md), `x` and **this**.",
      "https://github.com/o/r/blob/main/docs/a/b.md",
    )
    |> html.span([], _)
    |> element.to_string
  assert string.contains(
    html,
    "<a href=\"https://github.com/o/r/blob/main/docs/plan.md\">the <code>plan</code></a>",
  )
  assert string.contains(html, "<code>x</code>")
  assert string.contains(html, "<strong>this</strong>")
  assert string.starts_with(html, "<span>See ")
}

pub fn markdown_escapes_text_test() {
  let html =
    markdown.inline("<script>", "https://example.org/")
    |> html.span([], _)
    |> element.to_string
  assert !string.contains(html, "<script>")
}

pub fn format_test() {
  assert format.thousands(1_234_567) == "1,234,567"
  assert format.thousands(12) == "12"
  assert format.size(512) == "512 KB"
  assert format.size(1536) == "1.5 MB"
  assert format.months("2024-11", "2025-02")
    == ["2024-11", "2024-12", "2025-01", "2025-02"]
  assert format.plural(1, "commit") == "1 commit"
  assert format.plural(2, "commit") == "2 commits"
}

pub fn history_json_decodes_test() {
  let body =
    "{\"repo\":\"https://github.com/h0ffmann/picasso\",\"labs\":{\"lint\":\"b\",\"cuda\":\"a\"},
      \"eras\":[{\"title\":\"t\",\"blurb\":\"b\",\"from\":\"2024-12-30\",\"to\":\"2025-01-02\",
      \"commits\":[{\"sha\":\"eb0fae1\",\"date\":\"2024-12-30\",\"subject\":\"s\",\"pr\":null,
      \"bot\":false,\"added\":1,\"deleted\":0}]}]}"
  let assert Ok(h) = data.parse_history(body)
  assert h.labs == [#("cuda", "a"), #("lint", "b")]
  let assert [era] = h.eras
  assert list.length(era.commits) == 1
}

const releases_json =
  "{\"repo\": \"https://github.com/h0ffmann/picasso\", \"version\": \"v0.1.0\",
 \"doi\": \"10.5281/zenodo.8\", \"unreleased\": [{\"title\": \"Fixes\", \"entries\": [{\"subject\": \"vm: x\", \"pr\": 99, \"breaking\": false}]}],
 \"releases\": [{\"tag\": \"v0.1.0\", \"date\": \"2026-10-12\", \"previous\": null, \"doi\": \"10.5281/zenodo.9\",
   \"groups\": [{\"title\": \"Features\", \"entries\": [{\"subject\": \"site: a changelog\", \"pr\": null, \"breaking\": true}]}]}]}"

pub fn releases_render_as_badge_and_changelog_test() {
  let assert Ok(r) = data.parse_releases(releases_json)
  let html =
    app.Model(..app.initial(), releases: app.Loaded(r))
    |> app.view
    |> element.to_string
  assert string.contains(
    html,
    "href=\"https://github.com/h0ffmann/picasso/releases/tag/v0.1.0\"",
  )
  assert string.contains(html, "Unreleased · 1 change on main")
  assert string.contains(html, "site: a changelog")
  assert string.contains(html, "href=\"#changelog\"")
  assert string.contains(html, "href=\"https://doi.org/10.5281/zenodo.8\"")
  assert string.contains(html, "v0.1.0 alone is ")
  assert string.contains(html, "doi:10.5281/zenodo.9")
}

pub fn releases_without_dois_still_parse_test() {
  let assert Ok(r) =
    data.parse_releases(
      "{\"repo\": \"r\", \"version\": null, \"unreleased\": [], \"releases\": []}",
    )
  assert r.doi == None
}

pub fn build_time_releases_are_prerendered_test() {
  let assert app.Loaded(r) = app.initial().releases
  let assert Some(doi) = r.doi
  assert string.contains(
    prerendered(),
    "href=\"https://doi.org/" <> doi <> "\"",
  )
}

pub fn story_sections_are_in_order_test() {
  let html = prerendered()
  let ids = [
    "what",
    "why",
    "users",
    "turn",
    "proof",
    "labs",
    "papers",
    "changelog",
    "history",
    "standings",
  ]
  let positions =
    list.map(ids, fn(id) {
      let assert [before, ..] = string.split(html, "id=\"" <> id <> "\"")
      string.length(before)
    })
  assert positions == list.sort(positions, int.compare)
}

pub fn story_numbers_come_from_the_ptbr_pdf_readme_test() {
  let html = prerendered()
  assert story.before.on_disk == "729 MiB"
  assert story.after.disk_times == "29×"
  assert story.after.download_times == "25×"
  assert string.contains(html, "25.2 MiB")
  assert string.contains(html, "8.1 MiB")
  assert string.contains(html, "206 MiB")
  assert list.length(story.consumers) == 14
}
