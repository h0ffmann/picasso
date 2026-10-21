//// The story of release 0.2.0, as data and views. Every fact comes from the repository
//// (`README.md`, `AGENTS.md`, `labs/ptbr-pdf/README.md`); a ⚠ marks a fact that was not checked.

import gleam/list
import lustre/attribute
import lustre/element.{type Element}
import lustre/element/html

const repo = "https://github.com/h0ffmann/picasso"

pub type Consumer {
  Consumer(repo: String, lab: String, how: String, pinned: String)
}

/// "Who uses picasso", checked on 2026-10-21 against each repository's default branch.
/// An empty repo continues the row above it.
pub const consumers = [
  Consumer("h0ffmann/ww3-gpu", "publisher", "flake input and Action", "52c5772"),
  Consumer("", "gleam-website", "flake input", "52c5772"),
  Consumer("", "pratico", "sparse git submodule", "4bd3b22"),
  Consumer(
    "h0ffmann/h0ffmann",
    "publisher",
    "flake input and Action",
    "0d55b07",
  ),
  Consumer("", "gleam-website", "flake input", "52c5772"),
  Consumer("h0ffmann/closed-source", "paper", "flake input", "2d6f8d8"),
  Consumer("", "gleam-website", "flake input", "68723ab"),
  Consumer("", "viz", "git submodule (⚠ sparse not checked)", "3d60c00"),
  Consumer(
    "h0ffmann/nashkell",
    "gleam-website, paper",
    "flake inputs",
    "52c5772",
  ),
  Consumer("marola-dev/marola", "lint", "flake input", "6421951"),
  Consumer("", "agentic", "flake input (a branch ref)", "d42ff99"),
  Consumer(
    "marola-dev/marola-research",
    "paper, gleam-website",
    "flake inputs",
    "52c5772",
  ),
  Consumer(
    "h0ffmann/gcp-agentic-architect",
    "publisher, lint, agentic",
    "flake inputs and an Action",
    "461413a",
  ),
  Consumer(
    "h0ffmann/ptbr-writer",
    "gleam-website, ptbr-pdf",
    "flake input, submodule, nix run (⚠ ptbr-pdf only on an unmerged branch)",
    "4bd3b22, dcacdde",
  ),
]

pub type Benchmark {
  Benchmark(
    name: String,
    on_disk: String,
    download: String,
    disk_times: String,
    download_times: String,
  )
}

pub const before = Benchmark("pandoc + XeLaTeX", "729 MiB", "206 MiB", "", "")

pub const after = Benchmark("Rust + Typst", "25.2 MiB", "8.1 MiB", "29×", "25×")

pub type Turn {
  Turn(title: String, text: String)
}

pub const turns = [
  Turn(
    "Python and Node became Rust",
    "The scripts of the repository are now the crates of tools/. The scripts of each lab are crates in labs/<lab>/tools/, built by the lab's own flake.",
  ),
  Turn(
    "The lab contract",
    "A lab has its own flake.nix, flake.lock, justfile, README.md and lab.json. It references nothing outside itself. Every lab exports devShells, lib.<system>.tools, packages and checks.",
  ),
  Turn("English only", "Commits, pull requests and issues are in English."),
]

pub type Paper {
  Paper(title: String, href: String, what: String)
}

pub const papers = [
  Paper(
    "Same inputs, same factory",
    "https://github.com/h0ffmann/picasso/blob/main/PICASSOPAPER.pdf",
    "The determinism whitepaper, coloured original.",
  ),
  Paper(
    "Same inputs, same factory, ASD-STE100",
    "https://github.com/h0ffmann/picasso/blob/main/WHITEPAPER.pdf",
    "The same paper in Simplified Technical English.",
  ),
  Paper(
    "picasso and Determinate Nix",
    "https://github.com/h0ffmann/picasso/blob/main/docs/studies/determinate-2026/study.pdf",
    "A study of Determinate Systems and picasso, with a 6-month plan.",
  ),
  Paper(
    "Site hosting plan",
    "plan/hosting-plan.pdf",
    "The plan for hosting this site.",
  ),
]

const input_snippet = "inputs.lint.url = \"github:h0ffmann/picasso?dir=labs/lint\";"

const submodule_snippet = "nix develop ./picasso/labs/pratico#ww3"

const action_snippet = "- uses: h0ffmann/picasso/labs/publisher@main"

fn head(id: String, title: String) -> Element(msg) {
  html.div([attribute.class("section-head")], [
    html.h2([attribute.id(id)], [html.text(title)]),
  ])
}

fn story(id: String, title: String, body: List(Element(msg))) -> Element(msg) {
  html.section([attribute.id(id), attribute.aria_labelledby(id <> "-h")], [
    head(id <> "-h", title),
    ..body
  ])
}

fn p(text: String) -> Element(msg) {
  html.p([attribute.class("sub")], [html.text(text)])
}

fn cards(items: List(#(String, String))) -> Element(msg) {
  html.ul(
    [attribute.class("labs")],
    list.map(items, fn(i) {
      html.li([], [
        html.span([attribute.class("card-title")], [html.text(i.0)]),
        html.p([], [html.text(i.1)]),
      ])
    }),
  )
}

pub fn what() -> Element(msg) {
  story("what", "What picasso is", [
    p(
      "picasso is a set of labs. A lab is a self-contained Nix flake with the tools for one job. Other repositories borrow a lab. The builds are deterministic, and everything is pinned.",
    ),
  ])
}

pub fn why() -> Element(msg) {
  story("why", "Why it matters", [
    cards([
      #("The same environment", "The same tools run on any machine and in CI."),
      #(
        "One pin",
        "A consumer pins one commit of picasso. Its flake.lock holds the exact revision.",
      ),
    ]),
  ])
}

pub fn users() -> Element(msg) {
  let row = fn(c: Consumer) {
    html.tr([], [
      html.td([attribute.class("name")], [
        case c.repo {
          "" -> html.text("")
          r ->
            html.a([attribute.href("https://github.com/" <> r)], [html.text(r)])
        },
      ]),
      html.td([], [html.code([], [html.text(c.lab)])]),
      html.td([], [html.text(c.how)]),
      html.td([attribute.class("opt")], [html.code([], [html.text(c.pinned)])]),
    ])
  }
  story("users", "Who uses picasso", [
    p(
      "Checked on 2026-10-21 against the default branch of each repository. The pin is the revision that the lock or the submodule holds. Private repositories are not listed. The lab present has no consumer: it is for this workstation.",
    ),
    html.div([attribute.class("table-scroll")], [
      html.table([attribute.class("repos")], [
        html.thead([], [
          html.tr(
            [],
            list.map(["Repository", "Lab", "How", "Pinned at"], fn(h) {
              html.th([], [html.text(h)])
            }),
          ),
        ]),
        html.tbody([], list.map(consumers, row)),
      ]),
    ]),
  ])
}

pub fn turn() -> Element(msg) {
  story("turn", "The 0.2.0 turn: only Nix and Rust", [
    p(
      "From 0.2.0, the code of the repository is Nix or Rust. The Gleam sites and one C++ smoke test are the only exceptions.",
    ),
    cards(list.map(turns, fn(t) { #(t.title, t.text) })),
  ])
}

fn stat(value: String, label: String) -> Element(msg) {
  html.div([attribute.class("stat")], [
    html.b([], [html.text(value)]),
    html.span([], [html.text(label)]),
  ])
}

pub fn proof() -> Element(msg) {
  story("proof", "One proof, in numbers", [
    p(
      "The lab ptbr-pdf turns Markdown into a PDF for Portuguese. It was pandoc and XeLaTeX. It is now one Rust binary with Typst inside.",
    ),
    html.div([attribute.class("stats")], [
      stat(before.on_disk, before.name <> ", on disk"),
      stat(before.download, before.name <> ", download"),
      stat(after.on_disk, after.name <> ", static, on disk"),
      stat(after.download, after.name <> ", download with xz"),
      stat(after.disk_times, "less on disk"),
      stat(after.download_times, "less to download"),
    ]),
    html.p([attribute.class("sub")], [
      html.text(
        "Measured on x86_64-linux, 2026-10-20 and 2026-10-21. The method and the platforms are in ",
      ),
      html.a(
        [
          attribute.href(
            repo <> "/tree/main/labs/ptbr-pdf#benchmark-from-pandoc-to-rust",
          ),
        ],
        [
          html.text("labs/ptbr-pdf"),
        ],
      ),
      html.text("."),
    ]),
  ])
}

pub fn use_a_lab() -> Element(msg) {
  let snippet = fn(title, code) {
    html.li([], [
      html.span([attribute.class("card-title")], [html.text(title)]),
      html.pre([], [html.code([], [html.text(code)])]),
    ])
  }
  html.div([attribute.class("use")], [
    html.h3([], [html.text("How to use a lab")]),
    html.ul([attribute.class("labs")], [
      snippet("A flake input", input_snippet),
      snippet("A sparse submodule", submodule_snippet),
      snippet("A GitHub Action", action_snippet),
    ]),
  ])
}

pub fn papers_section() -> Element(msg) {
  story("papers", "Papers and how to cite", [
    p("Each release has a DOI on Zenodo. The header shows the one to cite."),
    html.ul(
      [attribute.class("labs")],
      list.map(papers, fn(x) {
        html.li([], [
          html.a([attribute.href(x.href)], [html.text(x.title)]),
          html.p([], [html.text(x.what)]),
        ])
      }),
    ),
  ])
}
