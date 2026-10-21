//// Bindings to `picasso_site_ffi.mjs`.

/// GET `url` and hand the body as text to one of the callbacks. With `cache`, a response is
/// reused from localStorage for ten minutes.
@external(javascript, "../picasso_site_ffi.mjs", "get")
pub fn get(
  url: String,
  cache: Bool,
  on_ok: fn(String) -> Nil,
  on_error: fn(String) -> Nil,
) -> Nil

@external(javascript, "../picasso_site_ffi.mjs", "resolve_url")
pub fn resolve_url(href: String, base: String) -> String

/// "3 days ago", against the viewer's clock.
@external(javascript, "../picasso_site_ffi.mjs", "relative_time")
pub fn relative_time(iso: String) -> String

@external(javascript, "../picasso_site_ffi.mjs", "now_local")
pub fn now_local() -> String

/// The releases and DOIs as JSON, written by `site-releases` before the build.
@external(javascript, "../picasso_site_releases.mjs", "releases")
pub fn releases() -> String
