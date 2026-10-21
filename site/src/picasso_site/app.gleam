//// The page as one Lustre application. `view(initial())` is also what `prerender` writes into
//// `index.html`, so the page reads without JavaScript and the client takes it over in place.

import gleam/float
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/order
import gleam/regexp
import gleam/set
import gleam/string
import gleam/uri
import lustre/attribute.{type Attribute}
import lustre/effect.{type Effect}
import lustre/element.{type Element}
import lustre/element/html
import lustre/event
import picasso_site/data.{
  type Era, type Group, type History, type Releases, type Repo,
}
import picasso_site/drawings.{type Drawing}
import picasso_site/family
import picasso_site/ffi
import picasso_site/format
import picasso_site/markdown
import picasso_site/readiness.{type Readiness, type Status}
import picasso_site/story

const org = "ouroboric-dev"

const readiness_dir =
  "https://api.github.com/repos/ouroboric-dev/ouroboric/contents/docs/1-company/readiness"

const readiness_home =
  "https://github.com/ouroboric-dev/ouroboric/tree/main/docs/1-company/readiness"

pub type Remote(a) {
  Loading
  Loaded(a)
  Failed(String)
}

pub type Assessment {
  Assessment(html_url: String, readiness: Readiness)
}

pub type Sort {
  Pushed
  Stars
  Issues
  Size
}

pub type Model {
  Model(
    drawing: Drawing,
    history: Remote(History),
    releases: Remote(Releases),
    assessment: Remote(Assessment),
    repos: Remote(List(Repo)),
    filter: Option(Status),
    sort: Sort,
    bar_grown: Bool,
    pending: Int,
    updated: String,
  )
}

pub type Msg {
  Redraw
  GotHistory(Result(String, String))
  GotListing(Result(String, String))
  GotAssessment(String, Result(String, String))
  GotRepos(Result(String, String))
  FilterBy(Option(Status))
  SortBy(Sort)
  GrowBar
}

pub fn initial() -> Model {
  Model(
    drawing: drawings.first,
    history: Loading,
    releases: ffi.releases() |> data.parse_releases |> remote,
    assessment: Loading,
    repos: Loading,
    filter: None,
    sort: Pushed,
    bar_grown: False,
    pending: 3,
    updated: "",
  )
}

pub fn init(_: Nil) -> #(Model, Effect(Msg)) {
  #(
    initial(),
    effect.batch([
      effect.from(fn(dispatch) { dispatch(Redraw) }),
      get("history.json", False, GotHistory),
      get(readiness_dir, True, GotListing),
      get(
        "https://api.github.com/orgs/"
          <> org
          <> "/repos?per_page=100&type=public",
        True,
        GotRepos,
      ),
    ]),
  )
}

fn get(
  url: String,
  cache: Bool,
  to_msg: fn(Result(String, String)) -> Msg,
) -> Effect(Msg) {
  use dispatch <- effect.from
  ffi.get(url, cache, fn(body) { dispatch(to_msg(Ok(body))) }, fn(error) {
    dispatch(to_msg(Error(error)))
  })
}

fn remote(result: Result(a, String)) -> Remote(a) {
  case result {
    Ok(value) -> Loaded(value)
    Error(message) -> Failed(message)
  }
}

/// One of the three fetched sources has settled; stamp the footer once all have.
fn settled(model: Model) -> Model {
  let pending = model.pending - 1
  case pending {
    0 -> Model(..model, pending:, updated: "Updated " <> ffi.now_local())
    _ -> Model(..model, pending:)
  }
}

pub fn update(model: Model, msg: Msg) -> #(Model, Effect(Msg)) {
  case msg {
    Redraw -> #(
      Model(..model, drawing: drawings.other_than(model.drawing)),
      effect.none(),
    )

    GotHistory(result) -> {
      let history = result |> then(data.parse_history) |> remote
      #(settled(Model(..model, history:)), effect.none())
    }

    GotListing(Ok(body)) ->
      case data.parse_listing(body) |> then(latest) {
        Ok(#(html_url, download_url)) -> #(
          model,
          get(download_url, True, GotAssessment(html_url, _)),
        )
        Error(message) -> #(
          settled(Model(..model, assessment: Failed(message))),
          effect.none(),
        )
      }
    GotListing(Error(message)) -> #(
      settled(Model(..model, assessment: Failed(message))),
      effect.none(),
    )

    GotAssessment(html_url, result) -> {
      let assessment =
        result
        |> then(fn(md) {
          case readiness.parse(md) {
            readiness.Readiness(items: [], ..) ->
              Error(
                "Could not read the items table in " <> name_of(html_url) <> ".",
              )
            parsed -> Ok(Assessment(html_url:, readiness: parsed))
          }
        })
        |> remote
      // The bar fills from empty after the first paint, so its widths animate in.
      #(
        settled(Model(..model, assessment:)),
        effect.after_paint(fn(dispatch, _) { dispatch(GrowBar) }),
      )
    }

    GotRepos(result) -> {
      let repos =
        result
        |> then(data.parse_repos)
        |> then(fn(repos) {
          Ok(list.filter(repos, fn(r) { !r.archived && !r.fork }))
        })
        |> remote
      #(settled(Model(..model, repos:)), effect.none())
    }

    FilterBy(filter) -> #(Model(..model, filter:), effect.none())
    SortBy(sort) -> #(Model(..model, sort:), effect.none())
    GrowBar -> #(Model(..model, bar_grown: True), effect.none())
  }
}

fn then(
  result: Result(a, String),
  f: fn(a) -> Result(b, String),
) -> Result(b, String) {
  case result {
    Ok(value) -> f(value)
    Error(e) -> Error(e)
  }
}

fn name_of(url: String) -> String {
  string.split(url, "/")
  |> list.last
  |> option.from_result
  |> option.unwrap(url)
}

/// The assessments are dated file names, so the newest sorts last.
fn latest(files: List(data.File)) -> Result(#(String, String), String) {
  let newest =
    files
    |> list.filter_map(fn(f) {
      case string.ends_with(f.name, ".md"), f.download_url {
        True, Some(download_url) -> Ok(#(f.name, f.html_url, download_url))
        _, _ -> Error(Nil)
      }
    })
    |> list.sort(fn(a, b) { string.compare(b.0, a.0) })
  case newest {
    [#(_, html_url, download_url), ..] -> Ok(#(html_url, download_url))
    [] -> Error("No readiness assessment found.")
  }
}

// VIEW ------------------------------------------------------------------------

pub fn view(model: Model) -> Element(Msg) {
  element.fragment([
    header(model.drawing, model.releases),
    html.main([attribute.class("wrap")], [
      credit(),
      family.band(),
      story.what(),
      story.why(),
      story.users(),
      story.turn(),
      story.proof(),
      labs_section(model.history),
      story.papers_section(),
      changelog_section(model.releases),
      history_section(model.history),
      standings(model),
    ]),
    footer(model.updated),
  ])
}

fn link(href: String, text: String) -> Element(msg) {
  html.a([attribute.href(href)], [html.text(text)])
}

fn header(drawing: Drawing, releases: Remote(Releases)) -> Element(Msg) {
  html.header([attribute.class("top")], [
    html.div([attribute.class("wrap hero")], [
      html.div([], [
        html.p([attribute.class("eyebrow")], [
          html.text("since 2024 · formerly nix-config"),
          version_badge(releases),
        ]),
        html.h1([], [html.text("picasso")]),
        html.p([attribute.class("lede")], [
          html.text(
            "Self-contained Nix flakes that pin scientific, publishing and agentic toolchains for other repositories. Since December 2024; only Nix and Rust since 0.2.0.",
          ),
        ]),
        cite(releases),
        html.ul([attribute.class("status"), attribute.aria_label("Status")], [
          status("is-live", "Nix VM ", "live"),
          status("is-soon", "NixOS development ", "soon"),
        ]),
        html.nav([attribute.class("jump")], [
          link("#users", "Who uses it"),
          link("#turn", "0.2.0"),
          link("#proof", "Proof"),
          link("#labs", "Labs"),
          link("#papers", "Papers"),
          link("#changelog", "Changelog"),
          link("#history", "History"),
          link("#standings", "ouroboric-dev standings"),
          link("plan/hosting-plan.pdf", "Hosting plan (PDF)"),
          link(
            "https://github.com/h0ffmann/picasso/blob/main/WHITEPAPER.pdf",
            "Whitepaper (PDF)",
          ),
          link(
            "https://github.com/h0ffmann/picasso/blob/main/PICASSOPAPER.pdf",
            "Picasso paper (PDF)",
          ),
          link("https://github.com/h0ffmann/picasso", "GitHub"),
        ]),
      ]),
      html.figure([attribute.class("drawing")], [
        html.img([
          attribute.id("drawing"),
          attribute.src(drawings.src(drawing)),
          attribute.alt(drawing.alt),
          attribute.width(360),
          attribute.height(240),
        ]),
        html.figcaption([], [
          html.text("Original drawing after Picasso, by this repo. "),
          html.button([attribute.type_("button"), event.on_click(Redraw)], [
            html.text("Another"),
          ]),
        ]),
      ]),
    ]),
  ])
}

fn status(class: String, label: String, state: String) -> Element(msg) {
  html.li([attribute.class(class)], [
    html.span([attribute.class("dot")], []),
    html.text(label),
    html.b([], [html.text(state)]),
  ])
}

fn credit() -> Element(msg) {
  let gvolpe = "https://github.com/gvolpe/nix-config"
  html.aside([attribute.class("credit")], [
    html.p([], [
      html.strong([], [html.text("Standing on Gabriel Volpe's shoulders.")]),
      html.text(
        " The README layout, the one-table-per-environment idea and more than one habit here come from ",
      ),
      link(gvolpe, "gvolpe/nix-config"),
      html.text(". His repository is better anyway: "),
      link(gvolpe, "go read it first"),
      html.text("."),
    ]),
  ])
}

fn section_head(
  id: String,
  title: List(Element(msg)),
  source: List(Attribute(msg)),
  source_text: String,
) -> Element(msg) {
  html.div([attribute.class("section-head")], [
    html.h2([attribute.id(id)], title),
    html.a([attribute.class("source"), ..source], [html.text(source_text)]),
  ])
}

fn error(id: String, remote: Remote(a)) -> Element(msg) {
  case remote {
    Failed(message) ->
      html.p([attribute.class("error"), attribute.id(id)], [html.text(message)])
    _ -> element.none()
  }
}

// History ---------------------------------------------------------------------

fn history_section(history: Remote(History)) -> Element(msg) {
  let body = case history {
    Loaded(h) -> history_body(h)
    _ -> []
  }
  html.section(
    [attribute.id("history"), attribute.aria_labelledby("history-h")],
    list.flatten([
      [
        section_head(
          "history-h",
          [html.text("History")],
          [attribute.href("https://github.com/h0ffmann/picasso/commits/main")],
          "all commits",
        ),
      ],
      body,
      [error("history-error", history)],
    ]),
  )
}

fn history_body(h: History) -> List(Element(msg)) {
  let commits = list.flat_map(h.eras, fn(e) { e.commits })
  let prs =
    commits
    |> list.filter_map(fn(c) { option.to_result(c.pr, Nil) })
    |> set.from_list
  let sum = fn(field: fn(data.Commit) -> Int) {
    int.sum(list.map(commits, field))
  }
  let month = fn(c: data.Commit) { string.slice(c.date, 0, 7) }
  let first =
    list.first(commits)
    |> option.from_result
    |> option.map(month)
    |> option.unwrap("")
  let last =
    list.last(commits)
    |> option.from_result
    |> option.map(month)
    |> option.unwrap("")
  let stats = [
    #(int.to_string(list.length(commits)), "commits on main"),
    #(int.to_string(set.size(prs)), "pull requests merged"),
    #("+" <> format.thousands(sum(fn(c) { c.added })), "lines added"),
    #("−" <> format.thousands(sum(fn(c) { c.deleted })), "lines removed"),
    #(int.to_string(list.length(h.labs)), "labs"),
    #(first, "first commit"),
  ]
  [
    html.div(
      [attribute.class("stats")],
      list.map(stats, fn(s) {
        html.div([attribute.class("stat")], [
          html.b([], [html.text(s.0)]),
          html.span([], [html.text(s.1)]),
        ])
      }),
    ),
    months(list.map(commits, month), first, last),
    html.ol(
      [attribute.class("eras")],
      list.index_map(h.eras, fn(e, i) { era(h.repo, e, i) }),
    ),
  ]
}

fn months(dates: List(String), first: String, last: String) -> Element(msg) {
  let span = format.months(first, last)
  let counts =
    list.map(span, fn(m) { #(m, list.count(dates, fn(d) { d == m })) })
  let peak = list.fold(counts, 1, fn(acc, c) { int.max(acc, c.1) })
  html.div(
    [
      attribute.class("months"),
      attribute.role("img"),
      attribute.aria_label("Commits per month, " <> first <> " to " <> last),
    ],
    list.map(counts, fn(c) {
      let #(m, n) = c
      let height = case n {
        0 -> 0.0
        _ -> float.max(6.0, int.to_float(n * 100) /. int.to_float(peak))
      }
      html.span(
        [
          attribute.title(m <> ": " <> format.plural(n, "commit")),
          attribute.classes([#("is-zero", n == 0)]),
          attribute.style(
            "--h",
            float.to_string(float.to_precision(height, 2)) <> "%",
          ),
        ],
        [],
      )
    }),
  )
}

fn era(repo: String, e: Era, i: Int) -> Element(msg) {
  let when = case e.from == e.to {
    True -> e.from
    False -> e.from <> " → " <> e.to
  }
  let assert Ok(pr_suffix) = regexp.from_string(" \\(#\\d+\\)$")
  html.li([attribute.class("era")], [
    html.div([attribute.class("era-head")], [
      html.span([attribute.class("era-n")], [html.text(int.to_string(i + 1))]),
      html.div([], [
        html.h3([], [html.text(e.title)]),
        html.p([attribute.class("when")], [
          html.text(
            when <> " · " <> int.to_string(list.length(e.commits)) <> " commits",
          ),
        ]),
      ]),
    ]),
    html.p([], [html.text(e.blurb)]),
    html.details([], [
      html.summary([], [html.text("Every change")]),
      html.ul(
        [attribute.class("log")],
        list.reverse(e.commits)
          |> list.map(fn(c) {
            let ref = case c.pr {
              Some(n) ->
                link(
                  repo <> "/pull/" <> int.to_string(n),
                  "#" <> int.to_string(n),
                )
              None -> link(repo <> "/commit/" <> c.sha, c.sha)
            }
            html.li([attribute.classes([#("bot", c.bot)])], [
              html.span([attribute.class("date")], [html.text(c.date)]),
              html.span([attribute.class("what")], [
                html.text(regexp.replace(pr_suffix, c.subject, "")),
              ]),
              html.span([attribute.class("ref")], [ref]),
            ])
          }),
      ),
    ]),
  ])
}

// Releases --------------------------------------------------------------------

fn version_badge(releases: Remote(Releases)) -> Element(msg) {
  case releases {
    Loaded(data.Releases(repo:, version: Some(tag), ..)) ->
      html.a(
        [
          attribute.class("version"),
          attribute.href(repo <> "/releases/tag/" <> tag),
          attribute.title("Latest release"),
        ],
        [html.text(tag)],
      )
    _ -> element.none()
  }
}

fn doi_link(doi: String) -> Element(msg) {
  html.a([attribute.class("doi"), attribute.href("https://doi.org/" <> doi)], [
    html.span([], [html.text("DOI")]),
    html.span([], [html.text(doi)]),
  ])
}

/// The concept DOI, and the latest release's own DOI once Zenodo has minted it.
fn cite(releases: Remote(Releases)) -> Element(msg) {
  case releases {
    Loaded(data.Releases(doi: Some(doi), version:, releases:, ..)) -> {
      let latest =
        list.find_map(releases, fn(rel) {
          case Some(rel.tag) == version, rel.doi {
            True, Some(own) -> Ok(own)
            _, _ -> Error(Nil)
          }
        })
      html.p([attribute.class("cite")], [
        doi_link(doi),
        html.text(" Cite this DOI: it always resolves to the latest release"),
        case latest, version {
          Ok(own), Some(tag) ->
            html.span([], [
              html.text(". " <> tag <> " alone is "),
              link("https://doi.org/" <> own, own),
            ])
          _, _ -> element.none()
        },
        html.text(". Archived on "),
        link("https://zenodo.org/doi/" <> doi, "Zenodo"),
        html.text("."),
      ])
    }
    _ -> element.none()
  }
}

fn changelog_section(releases: Remote(Releases)) -> Element(msg) {
  let body = case releases {
    Loaded(r) -> changelog_body(r)
    _ -> []
  }
  html.section(
    [attribute.id("changelog"), attribute.aria_labelledby("changelog-h")],
    list.flatten([
      [
        section_head(
          "changelog-h",
          [html.text("Changelog")],
          [attribute.href("https://github.com/h0ffmann/picasso/releases")],
          "releases",
        ),
        html.p([attribute.class("sub")], [
          html.text(
            "Semantic versions. A break in a lab's exported shape is a major bump. A new lab, recipe or page is a minor bump. A fix or a document is a patch. Before 1.0.0, a break is a minor bump. ",
          ),
          html.code([], [html.text("just release")]),
          html.text(" tags main, and the site deploys again from the tag."),
        ]),
      ],
      body,
      [error("changelog-error", releases)],
    ]),
  )
}

fn changelog_body(r: Releases) -> List(Element(msg)) {
  let unreleased = case r.unreleased {
    [] -> []
    groups -> [
      html.details([attribute.class("release")], [
        html.summary([], [
          html.text(
            "Unreleased · "
            <> format.plural(count_entries(groups), "change")
            <> " on main",
          ),
        ]),
        ..groups_view(r.repo, groups)
      ]),
    ]
  }
  let released =
    list.index_map(r.releases, fn(rel, i) {
      let compare = case rel.previous {
        Some(prev) -> r.repo <> "/compare/" <> prev <> "..." <> rel.tag
        None -> r.repo <> "/commits/" <> rel.tag
      }
      html.details([attribute.class("release"), attribute.open(i == 0)], [
        html.summary([], [
          html.b([], [html.text(rel.tag)]),
          html.text(
            " · "
            <> rel.date
            <> " · "
            <> format.plural(count_entries(rel.groups), "change"),
          ),
        ]),
        html.p([attribute.class("when")], [
          link(r.repo <> "/releases/tag/" <> rel.tag, "release notes"),
          html.text(" · "),
          link(compare, "diff"),
          case rel.doi {
            Some(doi) ->
              html.span([], [
                html.text(" · "),
                link("https://doi.org/" <> doi, "doi:" <> doi),
              ])
            None -> element.none()
          },
        ]),
        ..groups_view(r.repo, rel.groups)
      ])
    })
  case unreleased, released {
    [], [] -> [
      html.p([attribute.class("sub")], [html.text("No release yet.")]),
    ]
    _, _ -> list.append(unreleased, released)
  }
}

fn count_entries(groups: List(Group)) -> Int {
  int.sum(list.map(groups, fn(g) { list.length(g.entries) }))
}

fn groups_view(repo: String, groups: List(Group)) -> List(Element(msg)) {
  list.flat_map(groups, fn(g) {
    [
      html.h4([], [html.text(g.title)]),
      html.ul(
        [attribute.class("log")],
        list.map(g.entries, fn(e) {
          let ref = case e.pr {
            Some(n) ->
              link(
                repo <> "/pull/" <> int.to_string(n),
                "#" <> int.to_string(n),
              )
            None -> element.none()
          }
          html.li([], [
            html.span([attribute.class("what")], [
              case e.breaking {
                True -> html.b([], [html.text("breaking: ")])
                False -> element.none()
              },
              html.text(e.subject),
            ]),
            html.span([attribute.class("ref")], [ref]),
          ])
        }),
      ),
    ]
  })
}

fn labs_section(history: Remote(History)) -> Element(msg) {
  let labs = case history {
    Loaded(h) ->
      list.map(h.labs, fn(lab) {
        html.li([], [
          link(h.repo <> "/tree/main/labs/" <> uri.percent_encode(lab.0), lab.0),
          html.p([], [html.text(lab.1)]),
        ])
      })
    _ -> []
  }
  html.section([attribute.id("labs"), attribute.aria_labelledby("labs-h")], [
    section_head(
      "labs-h",
      [html.text("Labs")],
      [attribute.href("https://github.com/h0ffmann/picasso/tree/main/labs")],
      "labs/",
    ),
    html.p([attribute.class("sub")], [
      html.text(
        "Self-contained flakes: one directory each, nothing shared, borrowed as a flake input, a submodule or an Action.",
      ),
    ]),
    html.ul([attribute.class("labs")], labs),
    story.use_a_lab(),
  ])
}

// Standings -------------------------------------------------------------------

fn standings(model: Model) -> Element(Msg) {
  let source = case model.assessment {
    Loaded(a) -> a.html_url
    _ -> readiness_home
  }
  let title = case model.assessment {
    Loaded(Assessment(readiness: r, ..)) ->
      case r.bottom {
        "" -> r.title
        bottom -> r.title <> " — " <> bottom
      }
    _ -> ""
  }
  html.section(
    [attribute.id("standings"), attribute.aria_labelledby("readiness-h")],
    list.flatten([
      [
        section_head(
          "readiness-h",
          [
            html.img([
              attribute.class("mark"),
              attribute.src(family.ouroboric_icon),
              attribute.alt(""),
              attribute.width(28),
              attribute.height(28),
            ]),
            html.text(" ouroboric-dev standings"),
          ],
          [attribute.href(source)],
          "source",
        ),
        html.p([attribute.class("sub")], [html.text(title)]),
      ],
      case model.assessment {
        Loaded(a) -> [
          scoreboard(a.readiness, model.bar_grown),
          filters(model.filter),
          items(a, model.filter),
        ]
        _ -> [html.ol([attribute.class("items")], [])]
      },
      [
        error("readiness-error", model.assessment),
        html.div([attribute.class("section-head minor")], [
          html.h3([], [html.text("Repositories")]),
          html.label([attribute.class("sort")], [
            html.text("Rank by "),
            html.select(
              [event.on_change(fn(v) { SortBy(sort_from(v)) })],
              list.map(
                [
                  #(Pushed, "latest push"),
                  #(Stars, "stars"),
                  #(Issues, "open issues"),
                  #(Size, "size"),
                ],
                fn(o) {
                  html.option(
                    [
                      attribute.value(sort_name(o.0)),
                      attribute.selected(o.0 == model.sort),
                    ],
                    o.1,
                  )
                },
              ),
            ),
          ]),
        ]),
        repos_table(model.repos, model.sort),
        error("repos-error", model.repos),
      ],
    ]),
  )
}

fn scoreboard(r: Readiness, grown: Bool) -> Element(msg) {
  let counts =
    list.map(readiness.statuses, fn(s) { #(s, readiness.count(r.items, s)) })
  let label =
    counts
    |> list.map(fn(c) {
      int.to_string(c.1) <> " " <> readiness.status_name(c.0)
    })
    |> string.join(", ")
  html.div([attribute.class("scoreboard")], [
    html.div([attribute.class("score")], [
      html.span([attribute.class("big")], [
        html.text(int.to_string(readiness.count(r.items, readiness.Met))),
      ]),
      html.span([attribute.class("of")], [
        html.text("/ " <> int.to_string(list.length(r.items))),
      ]),
      html.span([attribute.class("label")], [html.text("readiness items met")]),
    ]),
    html.div(
      [
        attribute.class("bar"),
        attribute.role("img"),
        attribute.aria_label(label),
      ],
      list.map(counts, fn(c) {
        let grow = case grown {
          True -> c.1
          False -> 0
        }
        html.span(
          [
            attribute.class("is-" <> readiness.status_name(c.0)),
            attribute.style("flex-grow", int.to_string(grow)),
          ],
          [],
        )
      }),
    ),
    html.ul(
      [attribute.class("legend")],
      list.map(counts, fn(c) {
        let name = readiness.status_name(c.0)
        html.li([], [
          html.span([attribute.class("dot is-" <> name)], []),
          html.b([], [html.text(int.to_string(c.1))]),
          html.text(" " <> name),
        ])
      }),
    ),
  ])
}

fn filters(current: Option(Status)) -> Element(Msg) {
  let button = fn(filter: Option(Status), label: String) {
    html.button(
      [
        attribute.type_("button"),
        attribute.aria_pressed(case filter == current {
          True -> "true"
          False -> "false"
        }),
        event.on_click(FilterBy(filter)),
      ],
      [html.text(label)],
    )
  }
  html.div(
    [
      attribute.class("filters"),
      attribute.role("toolbar"),
      attribute.aria_label("Filter by status"),
    ],
    [
      button(None, "All"),
      button(Some(readiness.Missing), "Missing"),
      button(Some(readiness.Partial), "Partial"),
      button(Some(readiness.Met), "Met"),
    ],
  )
}

fn items(a: Assessment, filter: Option(Status)) -> Element(msg) {
  let base = a.html_url
  html.ol(
    [attribute.class("items")],
    readiness.sorted(a.readiness.items)
      |> list.map(fn(i) {
        let shown = case filter {
          None -> True
          Some(status) -> status == i.status
        }
        html.li(
          [
            attribute.class("item"),
            attribute.data("status", readiness.status_name(i.status)),
            attribute.hidden(!shown),
          ],
          [
            html.span([attribute.class("id")], [html.text(i.id)]),
            html.span([attribute.class("q")], markdown.inline(i.question, base)),
            html.span([attribute.class("pill")], [
              html.text(readiness.status_name(i.status)),
            ]),
            case i.to_close {
              "" -> element.none()
              text ->
                html.p([attribute.class("todo")], markdown.inline(text, base))
            },
            html.details([], [
              html.summary([], [html.text("Evidence")]),
              html.p([], markdown.inline(i.evidence, base)),
            ]),
          ],
        )
      }),
  )
}

fn sort_name(sort: Sort) -> String {
  case sort {
    Pushed -> "pushed"
    Stars -> "stars"
    Issues -> "issues"
    Size -> "size"
  }
}

fn sort_from(name: String) -> Sort {
  case name {
    "stars" -> Stars
    "issues" -> Issues
    "size" -> Size
    _ -> Pushed
  }
}

/// Newest push first; ISO 8601 timestamps sort as strings.
fn by_push(a: Repo, b: Repo) -> order.Order {
  string.compare(b.pushed_at, a.pushed_at)
}

pub fn sort_repos(repos: List(Repo), sort: Sort) -> List(Repo) {
  list.sort(repos, fn(a, b) {
    case sort {
      Pushed -> by_push(a, b)
      Stars -> int.compare(b.stars, a.stars) |> order.break_tie(by_push(a, b))
      Issues ->
        int.compare(b.issues, a.issues) |> order.break_tie(by_push(a, b))
      Size -> int.compare(b.size, a.size)
    }
  })
}

fn repos_table(repos: Remote(List(Repo)), sort: Sort) -> Element(msg) {
  let rows = case repos {
    Loaded(repos) -> sort_repos(repos, sort) |> list.index_map(repo_row)
    _ -> []
  }
  let th = fn(class: String, text: String) {
    html.th([attribute.class(class)], [html.text(text)])
  }
  html.div([attribute.class("table-scroll")], [
    html.table([attribute.class("repos")], [
      html.thead([], [
        html.tr([], [
          html.th([], [html.text("#")]),
          html.th([], [html.text("Repository")]),
          th("num", "★"),
          th("num", "Issues"),
          th("num opt", "Size"),
          html.th([], [html.text("Last push")]),
        ]),
      ]),
      html.tbody([], rows),
    ]),
  ])
}

fn repo_row(r: Repo, i: Int) -> Element(msg) {
  let td = fn(class: String, text: String) {
    html.td([attribute.class(class)], [html.text(text)])
  }
  html.tr([], [
    td("rank", int.to_string(i + 1)),
    html.td([], [
      html.a([attribute.class("name"), attribute.href(r.html_url)], [
        html.text(r.name),
      ]),
      case r.description {
        Some(d) if d != "" -> html.div([attribute.class("desc")], [html.text(d)])
        _ -> element.none()
      },
    ]),
    td("num", int.to_string(r.stars)),
    td("num", int.to_string(r.issues)),
    td("num opt", format.size(r.size)),
    html.td([attribute.class("when"), attribute.title(r.pushed_at)], [
      html.text(ffi.relative_time(r.pushed_at)),
    ]),
  ])
}

fn footer(updated: String) -> Element(msg) {
  html.footer([attribute.class("wrap foot")], [
    html.p([], [
      html.text("History from "),
      html.code([], [html.text("site/static/history.json")]),
      html.text(" ("),
      html.code([], [html.text("site/history.nix")]),
      html.text(
        "); changelog from the release tags and DOIs from Zenodo, both at build time (",
      ),
      html.code([], [html.text("tools/site-releases")]),
      html.text(
        "); standings from the public GitHub API, fetched in your browser and cached for ten minutes. ",
      ),
      html.span([], [html.text(updated)]),
    ]),
    html.p([], [
      html.text("Readiness items quoted from "),
      link(
        "https://github.com/ouroboric-dev/ouroboric",
        "ouroboric-dev/ouroboric",
      ),
      html.text(", © 2026 Bruno Guilhermo de Barros Valério, "),
      link("https://creativecommons.org/licenses/by/4.0/", "CC BY 4.0"),
      html.text(
        ", shown unchanged. The Marola and Ouroboric logos belong to their projects and are loaded from their own repositories to identify them; they are not covered by this repository's licences. ",
      ),
      link("https://github.com/h0ffmann", "Matheus Hoffmann"),
      html.text(", who keeps picasso, is a member of "),
      link("https://github.com/ouroboric-dev", "ouroboric-dev"),
      html.text("."),
    ]),
    html.p([], [
      html.text(
        "The drawings are original works made for this repository, in homage to Pablo Picasso's one-line drawings; they are not reproductions of his work. \"Picasso\" names this repository only and implies no affiliation with the Picasso estate. Code: ",
      ),
      link("https://github.com/h0ffmann/picasso/blob/main/LICENSE", "AGPL-3.0"),
      html.text("; drawings and text: "),
      link(
        "https://github.com/h0ffmann/picasso/blob/main/LICENSES/CC-BY-4.0.txt",
        "CC BY 4.0",
      ),
      html.text(". Built with "),
      link("https://gleam.run", "Gleam"),
      html.text(" and "),
      link("https://lustre.build", "Lustre"),
      html.text("."),
    ]),
  ])
}
