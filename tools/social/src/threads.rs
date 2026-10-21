//! The Threads Graph API: containers, carousel, publish. The token is never printed or logged.

use crate::post::{PostError, Result};
use serde_json::Value;
use std::{collections::BTreeMap, time::Duration};

pub const API: &str = "https://graph.threads.net/v1.0";
pub type Params = BTreeMap<String, String>;

pub fn params<const N: usize>(kv: [(&str, &str); N]) -> Params {
    kv.into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// One call to the API; `Http` is the real one, the self-test swaps in a fake.
pub trait Api {
    fn call(&mut self, method: &str, path: &str, params: &Params) -> Result<Value>;
}

pub struct Http {
    base: String,
    token: String,
    agent: ureq::Agent,
}

impl Http {
    pub fn new(base: &str, token: &str) -> Self {
        Http {
            base: base.trim_end_matches('/').into(),
            token: token.into(),
            agent: agent(),
        }
    }
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(60)))
        .build()
        .into()
}

impl Api for Http {
    fn call(&mut self, method: &str, path: &str, params: &Params) -> Result<Value> {
        let url = format!("{}/{path}", self.base);
        let mut all: Vec<(&str, &str)> = params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        all.push(("access_token", &self.token));
        let sent = if method == "GET" {
            self.agent.get(&url).query_pairs(all).call()
        } else {
            self.agent.post(&url).send_form(all)
        };
        // the URL (GET) and the body (POST) hold the token, so no error text may carry it
        let mut resp = sent.map_err(|e| {
            PostError(format!(
                "{method} {path}: {}",
                e.to_string().replace(&self.token, "***")
            ))
        })?;
        let status = resp.status().as_u16();
        let body = resp.body_mut().read_to_string().unwrap_or_default();
        if status >= 400 {
            // the body says why (expired token, link limit, ...)
            return Err(PostError(format!(
                "{method} {path}: HTTP {status} {}",
                body.replace(&self.token, "***")
            )));
        }
        serde_json::from_str(&body)
            .map_err(|e| PostError(format!("{method} {path}: bad JSON ({e})")))
    }
}

pub struct Threads<A: Api> {
    pub api: A,
    pub user: String,
    pub wait: Duration,
    pub tries: u32,
}

impl<A: Api> Threads<A> {
    pub fn new(api: A, user: &str, wait: Duration) -> Self {
        Threads {
            api,
            user: user.into(),
            wait,
            tries: 60,
        }
    }

    fn id(v: &Value) -> Result<String> {
        v["id"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| PostError(format!("no id in the reply: {v}")))
    }

    fn container(&mut self, p: Params) -> Result<String> {
        let r = self
            .api
            .call("POST", &format!("{}/threads", self.user), &p)?;
        let cid = Self::id(&r)?;
        // Meta asks for a wait until the container is processed before it is published.
        for _ in 0..self.tries {
            let st = self
                .api
                .call("GET", &cid, &params([("fields", "status,error_message")]))?;
            match st["status"].as_str() {
                Some("FINISHED") => return Ok(cid),
                Some(s @ ("ERROR" | "EXPIRED")) => {
                    return Err(PostError(format!(
                        "container {cid}: {s} {}",
                        st["error_message"].as_str().unwrap_or("")
                    )));
                }
                _ => std::thread::sleep(self.wait),
            }
        }
        Err(PostError(format!(
            "container {cid} not FINISHED after {} checks",
            self.tries
        )))
    }

    pub fn publish(
        &mut self,
        text: &str,
        images: &[(String, String)],
        link: &str,
    ) -> Result<String> {
        let cid = match images {
            // a link preview card exists only on text posts
            [] => {
                let mut p = params([("media_type", "TEXT"), ("text", text)]);
                if !link.is_empty() {
                    p.insert("link_attachment".into(), link.into());
                }
                self.container(p)?
            }
            [(url, alt)] => self.container(params([
                ("media_type", "IMAGE"),
                ("image_url", url),
                ("alt_text", alt),
                ("text", text),
            ]))?,
            _ => {
                let mut children = Vec::new();
                for (url, alt) in images {
                    children.push(self.container(params([
                        ("media_type", "IMAGE"),
                        ("image_url", url),
                        ("alt_text", alt),
                        ("is_carousel_item", "true"),
                    ]))?);
                }
                self.container(params([
                    ("media_type", "CAROUSEL"),
                    ("children", &children.join(",")),
                    ("text", text),
                ]))?
            }
        };
        let r = self.api.call(
            "POST",
            &format!("{}/threads_publish", self.user),
            &params([("creation_id", &cid)]),
        )?;
        let mid = Self::id(&r)?;
        let r = self
            .api
            .call("GET", &mid, &params([("fields", "permalink")]))?;
        Ok(r["permalink"]
            .as_str()
            .map_or_else(|| format!("media {mid}"), String::from))
    }
}

pub fn client() -> Result<Threads<Http>> {
    let user = std::env::var("THREADS_USER_ID").unwrap_or_default();
    let token = std::env::var("THREADS_ACCESS_TOKEN").unwrap_or_default();
    if user.is_empty() || token.is_empty() {
        return Err(PostError(
            "THREADS_USER_ID and THREADS_ACCESS_TOKEN must be set (repository secrets)".into(),
        ));
    }
    Ok(Threads::new(
        Http::new(API, &token),
        &user,
        Duration::from_secs(5),
    ))
}

/// Prints a new long-lived token on stdout (to be piped into `gh secret set`) and the validity on stderr.
pub fn refresh_at(base: &str, token: &str) -> Result<(String, u64)> {
    let url = format!("{}/refresh_access_token", base.trim_end_matches('/'));
    let mut resp = agent()
        .get(&url)
        .query_pairs([("grant_type", "th_refresh_token"), ("access_token", token)])
        .call()
        .map_err(|e| PostError(format!("refresh: {}", e.to_string().replace(token, "***"))))?;
    let status = resp.status().as_u16();
    let body = resp
        .body_mut()
        .read_to_string()
        .unwrap_or_default()
        .replace(token, "***");
    if status >= 400 {
        return Err(PostError(format!("refresh: HTTP {status} {body}")));
    }
    let v: Value =
        serde_json::from_str(&body).map_err(|e| PostError(format!("refresh: bad JSON ({e})")))?;
    let new = v["access_token"]
        .as_str()
        .ok_or_else(|| PostError("refresh: no access_token in the reply".into()))?;
    let secs = v["expires_in"]
        .as_u64()
        .ok_or_else(|| PostError("refresh: no expires_in in the reply".into()))?;
    Ok((new.into(), secs / 86400))
}

pub fn refresh() -> Result<i32> {
    let token = std::env::var("THREADS_ACCESS_TOKEN").unwrap_or_default();
    if token.is_empty() {
        return Err(PostError("THREADS_ACCESS_TOKEN must be set".into()));
    }
    let (new, days) = refresh_at("https://graph.threads.net", &token)?;
    eprintln!("social: new token valid for {days} days");
    println!("{new}");
    Ok(0)
}
