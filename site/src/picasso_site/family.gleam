//// Hard rule (AGENTS.md): the page always shows the Marola and Ouroboric logos, hotlinked from
//// their own repositories at a pinned commit, never copied here. `site-check` fails CI without
//// them. Bump a commit when a project changes its logo.

import lustre/attribute
import lustre/element.{type Element}
import lustre/element/html

pub const marola_logo =
  "https://raw.githubusercontent.com/marola-dev/marola/d0d576ea014335acbc190a09ce9fcbe0f89a391d/docs/assets/logo.svg"

const ouroboric =
  "https://raw.githubusercontent.com/ouroboric-dev/ouroboric/4ab80ed2486a5e0ab0ade13e5089c2a4acdb4216/docs/7-branding/identity/logo/"

pub const ouroboric_logo = ouroboric <> "ouroboric-logo-horizontal.svg"

pub const ouroboric_logo_dark = ouroboric <> "ouroboric-logo-horizontal-dark.svg"

pub const ouroboric_icon = ouroboric <> "ouroboric-icon.svg"

pub fn band() -> Element(msg) {
  html.section([attribute.class("family"), attribute.aria_label("Projects")], [
    html.a(
      [
        attribute.class("logo"),
        attribute.data("logo", "marola"),
        attribute.href("https://marola.dev"),
      ],
      [
        html.img([
          attribute.src(marola_logo),
          attribute.alt("Marola logo"),
          attribute.width(56),
          attribute.height(54),
        ]),
        html.span([], [
          html.b([], [html.text("marola")]),
          html.text("open ocean science, live at marola.dev"),
        ]),
      ],
    ),
    html.a(
      [
        attribute.class("logo"),
        attribute.data("logo", "ouroboric"),
        attribute.href("https://github.com/ouroboric-dev"),
      ],
      [
        html.picture([], [
          html.source([
            attribute.media("(prefers-color-scheme: dark)"),
            attribute.srcset(ouroboric_logo_dark),
          ]),
          html.img([
            attribute.src(ouroboric_logo),
            attribute.alt("Ouroboric logo"),
            attribute.height(44),
          ]),
        ]),
        html.span([], [html.text("open factories that improve themselves")]),
      ],
    ),
  ])
}
