use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use plugin_toolkit::anyhow::{anyhow, bail, Context, Result};
use plugin_toolkit::client::{Client, Request};
use plugin_toolkit::serde::{Deserialize, Serialize};
use plugin_toolkit::serde_json::{self, json, Value};
use plugin_toolkit::url;
use sha2::{Digest, Sha256};

const DEVICE_TYPE: &str = "A2CZJZGLK2JJVM";
const APP_VERSION: &str = "4.60";
const OS_VERSION: &str = "26.2";
const SOFTWARE_VERSION: &str = "46000856";
const DEVICE_NAME_TEMPLATE: &str =
    "%FIRST_NAME%%FIRST_NAME_POSSESSIVE_STRING%%DUPE_STRATEGY_1ST%Audible for iPhone";
const AUTH_CODE_PARAM: &str = "openid.oa2.authorization_code";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(crate = "plugin_toolkit::serde", rename_all = "lowercase")]
pub enum Marketplace {
    Us,
    Uk,
    De,
    Fr,
    Ca,
    It,
    Au,
    In,
    Jp,
    Es,
    Br,
}

impl Marketplace {
    pub fn tld(self) -> &'static str {
        match self {
            Self::Us => "com",
            Self::Uk => "co.uk",
            Self::De => "de",
            Self::Fr => "fr",
            Self::Ca => "ca",
            Self::It => "it",
            Self::Au => "com.au",
            Self::In => "in",
            Self::Jp => "co.jp",
            Self::Es => "es",
            Self::Br => "com.br",
        }
    }

    pub fn country_code(self) -> &'static str {
        match self {
            Self::Us => "us",
            Self::Uk => "uk",
            Self::De => "de",
            Self::Fr => "fr",
            Self::Ca => "ca",
            Self::It => "it",
            Self::Au => "au",
            Self::In => "in",
            Self::Jp => "jp",
            Self::Es => "es",
            Self::Br => "br",
        }
    }

    pub fn marketplace_id(self) -> &'static str {
        match self {
            Self::Us => "AF2M0KC94RCEA",
            Self::Uk => "A2I9A3Q2GNFNGQ",
            Self::De => "AN7V1F1VY261K",
            Self::Fr => "A2728XDNODOQ8T",
            Self::Ca => "A2CQZ5RBY40XE",
            Self::It => "A2N7FU2W2BU2ZC",
            Self::Au => "AN7EY7DTAW63G",
            Self::In => "AJO3FBRUE6J4S",
            Self::Jp => "A1QAP3MOU4173J",
            Self::Es => "ALMIKO4SZCSAR",
            Self::Br => "A10J1VAYUDTYRN",
        }
    }

    pub fn locale(self) -> &'static str {
        match self {
            Self::Us => "en-US",
            Self::Uk => "en-GB",
            Self::De => "de-DE",
            Self::Fr => "fr-FR",
            Self::Ca => "en-CA",
            Self::It => "it-IT",
            Self::Au => "en-AU",
            Self::In => "en-IN",
            Self::Jp => "ja-JP",
            Self::Es => "es-ES",
            Self::Br => "pt-BR",
        }
    }

    fn amazon(self) -> String {
        format!("https://www.amazon.{}", self.tld())
    }

    fn api(self) -> String {
        format!("https://api.amazon.{}", self.tld())
    }
}

#[derive(Clone)]
pub struct LoginRequest {
    pub marketplace: Marketplace,
    pub device_serial: String,
    pub client_id: String,
    code_verifier: String,
    pub code_challenge: String,
}

impl std::fmt::Debug for LoginRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoginRequest")
            .field("marketplace", &self.marketplace)
            .field("device_serial", &self.device_serial)
            .field("client_id", &self.client_id)
            .field("code_challenge", &self.code_challenge)
            .finish_non_exhaustive()
    }
}

impl LoginRequest {
    pub(crate) fn code_verifier(&self) -> &str {
        &self.code_verifier
    }

    pub fn new(marketplace: Marketplace) -> Result<Self> {
        let device_serial = device_serial()?;
        let mut verifier = [0u8; 32];
        getrandom::fill(&mut verifier).map_err(|e| anyhow!("rng: {e}"))?;
        let code_verifier = URL_SAFE_NO_PAD.encode(verifier);
        Ok(Self::from_parts(marketplace, device_serial, code_verifier))
    }

    fn from_parts(marketplace: Marketplace, device_serial: String, code_verifier: String) -> Self {
        let code_challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(code_verifier.as_bytes()));
        Self {
            marketplace,
            client_id: client_id(&device_serial),
            device_serial,
            code_verifier,
            code_challenge,
        }
    }

    pub fn login_url(&self) -> String {
        let m = self.marketplace;
        let client_id = format!("device:{}", self.client_id);
        let assoc_handle = format!("amzn_audible_ios_{}", m.country_code());
        let return_to = format!("{}/ap/maplanding", m.amazon());
        let params: [(&str, &str); 18] = [
            ("openid.oa2.response_type", "code"),
            ("openid.oa2.code_challenge_method", "S256"),
            ("openid.oa2.code_challenge", &self.code_challenge),
            ("openid.return_to", &return_to),
            ("openid.assoc_handle", &assoc_handle),
            (
                "openid.identity",
                "http://specs.openid.net/auth/2.0/identifier_select",
            ),
            ("pageId", "amzn_audible_ios"),
            ("accountStatusPolicy", "P1"),
            (
                "openid.claimed_id",
                "http://specs.openid.net/auth/2.0/identifier_select",
            ),
            ("openid.mode", "checkid_setup"),
            ("openid.ns.oa2", "http://www.amazon.com/ap/ext/oauth/2"),
            ("openid.oa2.client_id", &client_id),
            (
                "openid.ns.pape",
                "http://specs.openid.net/extensions/pape/1.0",
            ),
            ("marketPlaceId", m.marketplace_id()),
            ("openid.oa2.scope", "device_auth_access"),
            ("forceMobileLayout", "true"),
            ("openid.ns", "http://specs.openid.net/auth/2.0"),
            ("openid.pape.max_auth_age", "0"),
        ];
        format!("{}/ap/signin?{}", m.amazon(), query_string(&params))
    }
}

impl LoginRequest {
    pub fn is_success_redirect(&self, url: &str) -> bool {
        self.authorization_code(url).is_ok()
    }

    pub fn authorization_code(&self, url: &str) -> Result<String> {
        self.maplanding_query(url)
            .and_then(|q| query_param(q, AUTH_CODE_PARAM))
            .ok_or_else(|| anyhow!("{AUTH_CODE_PARAM} not in maplanding redirect url"))
    }

    // Exact base match pins scheme, host, port and path; the fragment is never parsed.
    fn maplanding_query<'a>(&self, url: &'a str) -> Option<&'a str> {
        let (base, rest) = url.split_once('?')?;
        if base != format!("{}/ap/maplanding", self.marketplace.amazon()) {
            return None;
        }
        rest.split('#').next()
    }
}

fn device_serial() -> Result<String> {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut out = String::with_capacity(40);
    let mut buf = [0u8; 64];
    while out.len() < 40 {
        getrandom::fill(&mut buf).map_err(|e| anyhow!("rng: {e}"))?;
        // Rejection sampling: 248 is the largest multiple of 62 below 256, so this stays unbiased.
        for b in buf.iter().filter(|b| **b < 248) {
            if out.len() == 40 {
                break;
            }
            out.push(ALPHABET[(*b % 62) as usize] as char);
        }
    }
    Ok(out)
}

fn client_id(serial: &str) -> String {
    format!("{serial}#{DEVICE_TYPE}")
        .bytes()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn query_string(params: &[(&str, &str)]) -> String {
    let owned: Vec<(String, String)> = params
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    url::query_string(&owned)
}

fn query_param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        (url::decode(k).ok()? == key).then(|| url::decode(v).ok())?
    })
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(crate = "plugin_toolkit::serde")]
pub struct Identity {
    pub access_token: String,
    /// Unix seconds.
    pub access_expires: f64,
    pub refresh_token: String,
    pub adp_token: String,
    pub device_private_key: String,
    pub website_cookies: BTreeMap<String, String>,
    pub store_authentication_cookie: Option<String>,
    pub device_serial: String,
    pub customer_id: String,
    pub device_name: String,
    pub marketplace: Marketplace,
}

impl std::fmt::Debug for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const R: &str = "<redacted>";
        f.debug_struct("Identity")
            .field("access_token", &R)
            .field("access_expires", &self.access_expires)
            .field("refresh_token", &R)
            .field("adp_token", &R)
            .field("device_private_key", &R)
            .field("website_cookies", &R)
            .field("store_authentication_cookie", &R)
            .field("device_serial", &self.device_serial)
            .field("customer_id", &self.customer_id)
            .field("device_name", &R)
            .field("marketplace", &self.marketplace)
            .finish()
    }
}

impl Identity {
    pub fn access_expired(&self, now: f64) -> bool {
        now >= self.access_expires - ACCESS_EXPIRY_MARGIN_SECS
    }

    /// The mkb79 `Authenticator.to_dict()` shape, which `LibationCli import-account` reads via `Mkb79Auth`.
    pub fn to_audible_cli_json(&self) -> Value {
        let website_cookies = if self.website_cookies.is_empty() {
            Value::Null
        } else {
            json!(self.website_cookies)
        };
        json!({
            "website_cookies": website_cookies,
            "adp_token": self.adp_token,
            "access_token": self.access_token,
            "refresh_token": self.refresh_token,
            "device_private_key": self.device_private_key,
            "store_authentication_cookie": self
                .store_authentication_cookie
                .as_ref()
                .map(|c| json!({ "cookie": c })),
            "device_info": {
                "device_name": self.device_name,
                "device_serial_number": self.device_serial,
                "device_type": DEVICE_TYPE,
            },
            "customer_info": { "user_id": self.customer_id },
            "expires": self.access_expires,
            "locale_code": self.marketplace.country_code(),
            "with_username": false,
            "activation_bytes": null,
        })
    }
}

fn secs_since_epoch(t: SystemTime) -> Result<f64> {
    t.duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .map_err(|_| anyhow!("system clock is before the unix epoch"))
}

fn register_body(code: &str, req: &LoginRequest) -> Value {
    json!({
        "requested_token_type": [
            "bearer",
            "mac_dms",
            "website_cookies",
            "store_authentication_cookie",
        ],
        "cookies": { "website_cookies": [], "domain": format!(".amazon.{}", req.marketplace.tld()) },
        "registration_data": {
            "domain": "Device",
            "app_version": APP_VERSION,
            "device_serial": req.device_serial,
            "device_type": DEVICE_TYPE,
            "device_name": DEVICE_NAME_TEMPLATE,
            "os_version": OS_VERSION,
            "software_version": SOFTWARE_VERSION,
            "device_model": "iPhone",
            "app_name": "Audible",
        },
        "auth_data": {
            "client_id": req.client_id,
            "authorization_code": code,
            "code_verifier": req.code_verifier(),
            "code_algorithm": "SHA-256",
            "client_domain": "DeviceLegacy",
        },
        "requested_extensions": ["device_info", "customer_info"],
    })
}

fn str_at<'a>(v: &'a Value, path: &[&str]) -> Result<&'a str> {
    path.iter()
        .try_fold(v, |acc, k| acc.get(k))
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("register response missing {}", path.join(".")))
}

fn expires_in(v: &Value) -> Result<f64> {
    match v {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.parse::<u64>().ok(),
        _ => None,
    }
    .filter(|&n| n > 0)
    .map(|n| n as f64)
    .ok_or_else(|| anyhow!("invalid expires_in: {v}"))
}

const ERROR_DETAIL_MAX: usize = 256;
/// Treat the access token as expired this many seconds early so in-flight requests do not race expiry.
const ACCESS_EXPIRY_MARGIN_SECS: f64 = 60.0;

// Amazon error bodies can echo token material; surface only the error fields, scrubbed and capped.
fn error_detail(body: &Value, code: &str, description: &str) -> Option<String> {
    let parts: Vec<&str> = [code, description]
        .iter()
        .filter_map(|p| body.pointer(p).and_then(Value::as_str))
        .collect();
    if parts.is_empty() {
        return None;
    }
    let scrubbed = plugin_toolkit::scrub::scrub_line(&parts.join(": ")).into_owned();
    Some(scrubbed.chars().take(ERROR_DETAIL_MAX).collect())
}

fn register_error(body: &Value) -> Option<String> {
    error_detail(
        body,
        "/response/error/code",
        "/response/error/error_description",
    )
}

fn refresh_error(body: &Value) -> Option<String> {
    error_detail(body, "/error", "/error_description")
}

fn parse_register(body: &Value, req: &LoginRequest, now: f64) -> Result<Identity> {
    let success = body.pointer("/response/success").ok_or_else(|| {
        anyhow!(
            "register failed: {}",
            register_error(body).unwrap_or_else(|| "unrecognized response".into())
        )
    })?;
    let tokens = &success["tokens"];
    let expires = expires_in(&tokens["bearer"]["expires_in"])?;
    let website_cookies = tokens["website_cookies"]
        .as_array()
        .map(|cookies| {
            cookies
                .iter()
                .filter_map(|c| {
                    Some((
                        c["Name"].as_str()?.to_string(),
                        c["Value"].as_str()?.replace('"', ""),
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    let store_authentication_cookie = tokens["store_authentication_cookie"]["cookie"]
        .as_str()
        .map(str::to_string);
    let extensions = &success["extensions"];
    Ok(Identity {
        access_token: str_at(tokens, &["bearer", "access_token"])?.to_string(),
        access_expires: now + expires,
        refresh_token: str_at(tokens, &["bearer", "refresh_token"])?.to_string(),
        adp_token: str_at(tokens, &["mac_dms", "adp_token"])?.to_string(),
        device_private_key: str_at(tokens, &["mac_dms", "device_private_key"])?.to_string(),
        website_cookies,
        store_authentication_cookie,
        device_serial: str_at(extensions, &["device_info", "device_serial_number"])?.to_string(),
        customer_id: str_at(extensions, &["customer_info", "user_id"])?.to_string(),
        device_name: str_at(extensions, &["device_info", "device_name"])?.to_string(),
        marketplace: req.marketplace,
    })
}

pub fn register(code: &str, req: &LoginRequest) -> Result<Identity> {
    let url = format!("{}/auth/register", req.marketplace.api());
    let resp = Client::new().post_json(url, &register_body(code, req))?;
    let body: Value = resp
        .json()
        .with_context(|| format!("register: HTTP {}", resp.status))?;
    if !resp.is_success() {
        match register_error(&body) {
            Some(detail) => bail!("register: HTTP {}: {detail}", resp.status),
            None => bail!("register: HTTP {}", resp.status),
        }
    }
    parse_register(&body, req, secs_since_epoch(SystemTime::now())?)
}

fn refresh_body(identity: &Identity) -> String {
    query_string(&[
        ("app_name", "Audible"),
        ("app_version", APP_VERSION),
        ("source_token", &identity.refresh_token),
        ("requested_token_type", "access_token"),
        ("source_token_type", "refresh_token"),
    ])
}

fn apply_refresh(identity: &Identity, body: &Value, now: f64) -> Result<Identity> {
    let access_token = body["access_token"]
        .as_str()
        .ok_or_else(|| anyhow!("token response missing access_token"))?;
    Ok(Identity {
        access_token: access_token.to_string(),
        access_expires: now + expires_in(&body["expires_in"])?,
        ..identity.clone()
    })
}

pub fn refresh(identity: &Identity) -> Result<Identity> {
    let url = format!("{}/auth/token", identity.marketplace.api());
    let req = Request::new("POST", url)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(refresh_body(identity));
    let resp = Client::new().send(req)?;
    if !resp.is_success() {
        match serde_json::from_slice::<Value>(&resp.body)
            .ok()
            .as_ref()
            .and_then(refresh_error)
        {
            Some(detail) => bail!("refresh: HTTP {}: {detail}", resp.status),
            None => bail!("refresh: HTTP {}", resp.status),
        }
    }
    let body: Value = serde_json::from_slice(&resp.body).context("refresh: invalid json")?;
    apply_refresh(identity, &body, secs_since_epoch(SystemTime::now())?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req() -> LoginRequest {
        LoginRequest::from_parts(
            Marketplace::Uk,
            "ABCdef0123456789ABCdef0123456789ABCdef01".into(),
            "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk".into(),
        )
    }

    fn register_fixture() -> Value {
        json!({
            "response": { "success": {
                "tokens": {
                    "bearer": { "access_token": "Atna|at", "refresh_token": "Atnr|rt", "expires_in": "3600" },
                    "mac_dms": { "adp_token": "{enc:x}", "device_private_key": "MIIEpk" },
                    "website_cookies": [
                        { "Name": "session-id", "Value": "\"123-456\"" },
                        { "Name": "ubid-acbuk", "Value": "789" }
                    ],
                    "store_authentication_cookie": { "cookie": "sac" }
                },
                "extensions": {
                    "device_info": {
                        "device_name": "Scott's Audible for iPhone",
                        "device_serial_number": "ABCdef0123456789ABCdef0123456789ABCdef01",
                        "device_type": "A2CZJZGLK2JJVM"
                    },
                    "customer_info": { "user_id": "amzn1.account.X", "given_name": "Scott" }
                }
            }}
        })
    }

    #[test]
    fn pkce_challenge_matches_rfc7636_vector() {
        assert_eq!(
            req().code_challenge,
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn client_id_is_hex_of_serial_and_device_type() {
        assert_eq!(client_id("AB"), "4142234132435a4a5a474c4b324a4a564d");
    }

    #[test]
    fn generated_request_shape() {
        let r = LoginRequest::new(Marketplace::Us).unwrap();
        assert_eq!(r.device_serial.len(), 40);
        assert!(r.device_serial.bytes().all(|b| b.is_ascii_alphanumeric()));
        assert_eq!(r.code_verifier().len(), 43);
    }

    #[test]
    fn login_url_params() {
        let r = req();
        let url = r.login_url();
        assert!(url.starts_with("https://www.amazon.co.uk/ap/signin?"));
        let p = |k| query_param(url.split_once('?').unwrap().1, k).unwrap();
        assert_eq!(p("openid.oa2.response_type"), "code");
        assert_eq!(p("openid.oa2.code_challenge_method"), "S256");
        assert_eq!(p("openid.oa2.code_challenge"), r.code_challenge);
        assert_eq!(p("openid.oa2.client_id"), format!("device:{}", r.client_id));
        assert_eq!(p("openid.oa2.scope"), "device_auth_access");
        assert_eq!(
            p("openid.return_to"),
            "https://www.amazon.co.uk/ap/maplanding"
        );
        assert_eq!(p("openid.assoc_handle"), "amzn_audible_ios_uk");
        assert_eq!(p("pageId"), "amzn_audible_ios");
        assert_eq!(p("accountStatusPolicy"), "P1");
        assert_eq!(
            p("openid.identity"),
            "http://specs.openid.net/auth/2.0/identifier_select"
        );
        assert_eq!(
            p("openid.claimed_id"),
            "http://specs.openid.net/auth/2.0/identifier_select"
        );
        assert_eq!(p("openid.mode"), "checkid_setup");
        assert_eq!(p("openid.ns"), "http://specs.openid.net/auth/2.0");
        assert_eq!(p("openid.ns.oa2"), "http://www.amazon.com/ap/ext/oauth/2");
        assert_eq!(
            p("openid.ns.pape"),
            "http://specs.openid.net/extensions/pape/1.0"
        );
        assert_eq!(p("openid.pape.max_auth_age"), "0");
        assert_eq!(p("marketPlaceId"), "A2I9A3Q2GNFNGQ");
        assert_eq!(p("forceMobileLayout"), "true");
    }

    #[test]
    fn code_extraction() {
        let ok = "https://www.amazon.com/ap/maplanding?openid.assoc_handle=x&openid.oa2.authorization_code=ANabc%2Bdef&openid.mode=id_res";
        let r = LoginRequest::from_parts(Marketplace::Us, "S".into(), "V".into());
        assert!(r.is_success_redirect(ok));
        assert_eq!(r.authorization_code(ok).unwrap(), "ANabc+def");
        let signin = "https://www.amazon.com/ap/signin?openid.mode=checkid_setup";
        assert!(!r.is_success_redirect(signin));
        assert!(r.authorization_code(signin).is_err());
        for bad in [
            "https://evil.example/x?y=/ap/maplanding&openid.oa2.authorization_code=Z",
            "https://www.amazon.com.evil.example/ap/maplanding?openid.oa2.authorization_code=Z",
            "https://www.amazon.co.uk/ap/maplanding?openid.oa2.authorization_code=Z",
            "https://www.amazon.com/x/ap/maplanding?openid.oa2.authorization_code=Z",
            "https://www.amazon.com/ap/maplanding/x?openid.oa2.authorization_code=Z",
            "http://www.amazon.com/ap/maplanding?openid.oa2.authorization_code=Z",
            "https://www.amazon.com/ap/maplanding?openid.mode=id_res#openid.oa2.authorization_code=Z",
            "https://www.amazon.com/ap/maplanding#?openid.oa2.authorization_code=Z",
        ] {
            assert!(!r.is_success_redirect(bad), "{bad}");
            assert!(r.authorization_code(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn register_body_matches_mkb79() {
        let b = register_body("CODE", &req());
        assert_eq!(b["cookies"]["domain"], ".amazon.co.uk");
        assert_eq!(b["auth_data"]["client_id"], req().client_id);
        assert_eq!(b["auth_data"]["authorization_code"], "CODE");
        assert_eq!(b["auth_data"]["code_verifier"], req().code_verifier());
        assert_eq!(b["auth_data"]["code_algorithm"], "SHA-256");
        assert_eq!(b["auth_data"]["client_domain"], "DeviceLegacy");
        assert_eq!(b["registration_data"]["device_serial"], req().device_serial);
        assert_eq!(
            b["requested_extensions"],
            json!(["device_info", "customer_info"])
        );
    }

    #[test]
    fn access_expired_applies_margin() {
        let id = parse_register(&register_fixture(), &req(), 1000.0).unwrap();
        assert!(!id.access_expired(1000.0));
        assert!(!id.access_expired(4539.0));
        assert!(id.access_expired(4540.0));
        assert!(id.access_expired(4600.0));
        assert!(id.access_expired(9000.0));
    }

    #[test]
    fn parses_register_response() {
        let id = parse_register(&register_fixture(), &req(), 1000.0).unwrap();
        assert_eq!(id.access_token, "Atna|at");
        assert_eq!(id.access_expires, 4600.0);
        assert_eq!(id.refresh_token, "Atnr|rt");
        assert_eq!(id.adp_token, "{enc:x}");
        assert_eq!(id.device_private_key, "MIIEpk");
        assert_eq!(id.website_cookies["session-id"], "123-456");
        assert_eq!(id.store_authentication_cookie.as_deref(), Some("sac"));
        assert_eq!(id.customer_id, "amzn1.account.X");
        assert_eq!(id.device_name, "Scott's Audible for iPhone");
        assert_eq!(id.marketplace, Marketplace::Uk);
    }

    #[test]
    fn register_requires_device_info_fields() {
        for field in ["device_name", "device_serial_number"] {
            let mut body = register_fixture();
            body["response"]["success"]["extensions"]["device_info"]
                .as_object_mut()
                .unwrap()
                .remove(field);
            let err = parse_register(&body, &req(), 0.0).unwrap_err().to_string();
            assert_eq!(
                err,
                format!("register response missing device_info.{field}")
            );
        }
    }

    #[test]
    fn register_error_surfaces() {
        let err = parse_register(
            &json!({"response": {"error": {"code": "InvalidValue", "leak": "Atna|planted"}}}),
            &req(),
            0.0,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("InvalidValue"), "{err}");
        assert!(!err.contains("Atna|planted"), "{err}");
    }

    #[test]
    fn refresh_error_extracts_only_error_fields() {
        let detail = refresh_error(&json!({
            "error": "invalid_grant",
            "error_description": "x".repeat(1000),
            "access_token": "Atna|planted",
        }))
        .unwrap();
        assert!(detail.starts_with("invalid_grant"), "{detail}");
        assert!(!detail.contains("Atna|planted"));
        assert_eq!(detail.chars().count(), ERROR_DETAIL_MAX);
        assert_eq!(
            refresh_error(&json!({"access_token": "Atna|planted"})),
            None
        );
    }

    #[test]
    fn refresh_replaces_access_token_only() {
        let id = parse_register(&register_fixture(), &req(), 0.0).unwrap();
        assert_eq!(
            refresh_body(&id),
            "app_name=Audible&app_version=4.60&source_token=Atnr%7Crt&requested_token_type=access_token&source_token_type=refresh_token"
        );
        let next = apply_refresh(
            &id,
            &json!({"access_token": "Atna|new", "expires_in": 3600}),
            50.0,
        )
        .unwrap();
        assert_eq!(next.access_token, "Atna|new");
        assert_eq!(next.access_expires, 3650.0);
        assert_eq!(next.refresh_token, id.refresh_token);
    }

    #[test]
    fn audible_cli_json_shape() {
        let id = parse_register(&register_fixture(), &req(), 1000.0).unwrap();
        let j = id.to_audible_cli_json();
        let mut keys: Vec<_> = j.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        assert_eq!(
            keys,
            [
                "access_token",
                "activation_bytes",
                "adp_token",
                "customer_info",
                "device_info",
                "device_private_key",
                "expires",
                "locale_code",
                "refresh_token",
                "store_authentication_cookie",
                "website_cookies",
                "with_username",
            ]
        );
        assert_eq!(j["locale_code"], "uk");
        assert_eq!(j["with_username"], false);
        assert_eq!(j["expires"], 4600.0);
        assert_eq!(j["store_authentication_cookie"], json!({"cookie": "sac"}));
        assert_eq!(j["website_cookies"]["session-id"], "123-456");
        assert_eq!(j["device_info"]["device_serial_number"], id.device_serial);
        assert_eq!(j["device_info"]["device_type"], "A2CZJZGLK2JJVM");
        assert_eq!(j["customer_info"]["user_id"], "amzn1.account.X");
    }

    fn mkb79_fixture() -> Value {
        let text = include_str!("testdata/mkb79_auth.jsonc");
        serde_json::from_str(text.split_once('\n').unwrap().1).unwrap()
    }

    fn json_type(v: &Value) -> &'static str {
        match v {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }

    // Keys whose Mkb79Auth.cs property is nullable (`JObject?` / `string?`) and may be emitted as null.
    const MKB79_NULLABLE: [&str; 3] = [
        "website_cookies",
        "store_authentication_cookie",
        "activation_bytes",
    ];

    fn assert_types_match(ours: &Value, golden: &Value, path: &str) {
        let (ours, golden) = (ours.as_object().unwrap(), golden.as_object().unwrap());
        for (k, v) in ours {
            let at = format!("{path}/{k}");
            let g = golden
                .get(k)
                .unwrap_or_else(|| panic!("{at} not in fixture"));
            if v.is_null() && MKB79_NULLABLE.contains(&k.as_str()) {
                continue;
            }
            assert_eq!(json_type(v), json_type(g), "{at}");
            if v.is_object() {
                assert_types_match(v, g, &at);
            }
        }
    }

    #[test]
    fn audible_cli_json_types_match_mkb79_fixture() {
        let golden = mkb79_fixture();
        let id = parse_register(&register_fixture(), &req(), 1000.0).unwrap();
        let ours = id.to_audible_cli_json();
        let mut ours_keys: Vec<_> = ours.as_object().unwrap().keys().collect();
        let mut golden_keys: Vec<_> = golden.as_object().unwrap().keys().collect();
        ours_keys.sort();
        golden_keys.sort();
        assert_eq!(ours_keys, golden_keys);
        assert_types_match(&ours, &golden, "");
        assert_eq!(
            ours["device_info"].as_object().unwrap().len(),
            golden["device_info"].as_object().unwrap().len()
        );

        let bare = Identity {
            website_cookies: BTreeMap::new(),
            store_authentication_cookie: None,
            ..id
        };
        let bare = bare.to_audible_cli_json();
        assert!(bare["website_cookies"].is_null());
        assert!(bare["store_authentication_cookie"].is_null());
        assert_types_match(&bare, &golden, "");
    }

    #[derive(Deserialize)]
    #[serde(crate = "plugin_toolkit::serde", deny_unknown_fields)]
    struct LibationDeviceInfo {
        device_name: Option<String>,
        device_serial_number: Option<String>,
        device_type: Option<String>,
    }

    #[derive(Deserialize)]
    #[serde(crate = "plugin_toolkit::serde", deny_unknown_fields)]
    struct LibationCustomerInfo {
        account_pool: Option<String>,
        user_id: Option<String>,
        home_region: Option<String>,
        name: Option<String>,
        given_name: Option<String>,
    }

    #[derive(Deserialize)]
    #[serde(crate = "plugin_toolkit::serde", deny_unknown_fields)]
    struct LibationStoreCookie {
        cookie: String,
    }

    // Mirrors Mkb79Auth.cs. Non-Option fields are the ones Libation cannot import without:
    // `expires`/`with_username` are non-nullable C# values, `ToAccountAsync` throws without
    // refresh/adp/private key, and `Locale` needs `locale_code`.
    #[derive(Deserialize)]
    #[serde(crate = "plugin_toolkit::serde", deny_unknown_fields)]
    struct LibationMkb79Auth {
        website_cookies: Option<BTreeMap<String, Option<String>>>,
        adp_token: String,
        access_token: Option<String>,
        refresh_token: String,
        device_private_key: String,
        store_authentication_cookie: Option<LibationStoreCookie>,
        device_info: Option<LibationDeviceInfo>,
        customer_info: Option<LibationCustomerInfo>,
        expires: f64,
        locale_code: String,
        with_username: bool,
        activation_bytes: Option<String>,
    }

    #[test]
    fn audible_cli_json_deserializes_as_libation_mkb79_auth() {
        let golden: LibationMkb79Auth = serde_json::from_value(mkb79_fixture()).unwrap();
        let gc = golden.customer_info.unwrap();
        assert_eq!(gc.account_pool.as_deref(), Some("Amazon"));
        assert_eq!(gc.home_region.as_deref(), Some("EU"));
        assert_eq!(gc.name.as_deref(), Some("Scott"));
        assert_eq!(gc.given_name.as_deref(), Some("Scott"));

        let id = parse_register(&register_fixture(), &req(), 1000.0).unwrap();
        let a: LibationMkb79Auth = serde_json::from_value(id.to_audible_cli_json()).unwrap();
        assert_eq!(a.access_token.as_deref(), Some(id.access_token.as_str()));
        assert_eq!(a.refresh_token, id.refresh_token);
        assert_eq!(a.adp_token, id.adp_token);
        assert_eq!(a.device_private_key, id.device_private_key);
        assert_eq!(a.expires, id.access_expires);
        assert_eq!(a.locale_code, "uk");
        assert!(!a.with_username);
        assert_eq!(a.activation_bytes, None);
        assert_eq!(a.store_authentication_cookie.unwrap().cookie, "sac");
        let cookies = a.website_cookies.unwrap();
        assert_eq!(cookies["session-id"].as_deref(), Some("123-456"));
        let d = a.device_info.unwrap();
        assert_eq!(d.device_name.as_deref(), Some(id.device_name.as_str()));
        assert_eq!(
            d.device_serial_number.as_deref(),
            Some(id.device_serial.as_str())
        );
        assert_eq!(d.device_type.as_deref(), Some(DEVICE_TYPE));
        let c = a.customer_info.unwrap();
        assert_eq!(c.user_id.as_deref(), Some(id.customer_id.as_str()));

        let bare = Identity {
            website_cookies: BTreeMap::new(),
            store_authentication_cookie: None,
            ..id
        };
        let b: LibationMkb79Auth = serde_json::from_value(bare.to_audible_cli_json()).unwrap();
        assert!(b.website_cookies.is_none());
        assert!(b.store_authentication_cookie.is_none());

        for m in [
            Marketplace::Us,
            Marketplace::Uk,
            Marketplace::De,
            Marketplace::Fr,
            Marketplace::Ca,
            Marketplace::It,
            Marketplace::Au,
            Marketplace::In,
            Marketplace::Jp,
            Marketplace::Es,
            Marketplace::Br,
        ] {
            assert!(
                ["de", "us", "uk", "fr", "ca", "it", "au", "in", "jp", "es", "br"]
                    .contains(&m.country_code()),
                "{m:?}"
            );
        }
    }

    #[test]
    fn debug_redacts_secrets() {
        let r = format!("{:?}", req());
        assert!(!r.contains("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"));
        let id = format!(
            "{:?}",
            parse_register(&register_fixture(), &req(), 1000.0).unwrap()
        );
        for secret in [
            "Atna|at",
            "Atnr|rt",
            "{enc:x}",
            "MIIEpk",
            "123-456",
            "ubid-acbuk",
            "sac",
        ] {
            assert!(!id.contains(secret), "{secret} leaked: {id}");
        }
        assert!(id.contains("amzn1.account.X"));
    }

    #[test]
    fn secs_since_epoch_rejects_pre_epoch_clock() {
        use std::time::Duration;
        assert_eq!(
            secs_since_epoch(UNIX_EPOCH + Duration::from_secs(1_700_000_000)).unwrap(),
            1_700_000_000.0
        );
        assert!(secs_since_epoch(UNIX_EPOCH - Duration::from_secs(1)).is_err());
    }

    #[test]
    fn expires_in_accepts_only_positive_integers() {
        for bad in [
            json!("NaN"),
            json!("inf"),
            json!("-1"),
            json!("3600.5"),
            json!("0"),
            json!(0),
            json!(-1),
            json!(3600.5),
        ] {
            assert!(expires_in(&bad).is_err(), "{bad} accepted");
        }
        assert_eq!(expires_in(&json!("3600")).unwrap(), 3600.0);
        assert_eq!(expires_in(&json!(3600)).unwrap(), 3600.0);
    }

    fn with_http<R>(
        status: u16,
        body: &[u8],
        call: impl FnOnce() -> R,
    ) -> (R, plugin_toolkit::abi::HttpRequest) {
        use plugin_toolkit::abi::{HttpRequest, HttpResponse};
        use std::cell::RefCell;
        use std::rc::Rc;
        let seen: Rc<RefCell<Option<HttpRequest>>> = Rc::default();
        let reply = serde_json::to_string(&HttpResponse {
            status,
            headers: vec![],
            body: body.to_vec(),
        })
        .unwrap();
        let sink_seen = seen.clone();
        let sink = Box::new(move |cap: &str, op: &str| {
            assert_eq!(cap, "http.request");
            *sink_seen.borrow_mut() = Some(serde_json::from_str(op).unwrap());
            Ok::<String, String>(reply.clone())
        });
        let out = plugin_toolkit::capsink::with_cap_sink(sink, call);
        let sent = seen.borrow_mut().take().expect("no request sent");
        (out, sent)
    }

    fn header<'a>(req: &'a plugin_toolkit::abi::HttpRequest, name: &str) -> Option<&'a str> {
        req.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    fn assert_register_request(sent: &plugin_toolkit::abi::HttpRequest) {
        assert_eq!(sent.method, "POST");
        assert_eq!(sent.url, "https://api.amazon.co.uk/auth/register");
        assert_eq!(header(sent, "content-type"), Some("application/json"));
        let body: Value = serde_json::from_slice(&sent.body).unwrap();
        assert_eq!(body, register_body("CODE", &req()));
    }

    fn assert_refresh_request(sent: &plugin_toolkit::abi::HttpRequest) {
        assert_eq!(sent.method, "POST");
        assert_eq!(sent.url, "https://api.amazon.co.uk/auth/token");
        assert_eq!(
            header(sent, "content-type"),
            Some("application/x-www-form-urlencoded")
        );
        assert_eq!(
            String::from_utf8(sent.body.clone()).unwrap(),
            "app_name=Audible&app_version=4.60&source_token=Atnr%7Crt&requested_token_type=access_token&source_token_type=refresh_token"
        );
    }

    #[test]
    fn register_http_success() {
        let body = serde_json::to_vec(&register_fixture()).unwrap();
        let (id, sent) = with_http(200, &body, || register("CODE", &req()));
        assert_register_request(&sent);
        let id = id.unwrap();
        assert_eq!(id.access_token, "Atna|at");
        assert_eq!(id.refresh_token, "Atnr|rt");
        assert_eq!(id.marketplace, Marketplace::Uk);
    }

    #[test]
    fn register_http_error_body_surfaces_code_not_tokens() {
        let body = serde_json::to_vec(&json!({"response": {"error": {
            "code": "InvalidValue",
            "error_description": "bad code",
            "leak": "Atna|planted",
        }}}))
        .unwrap();
        let (res, sent) = with_http(400, &body, || register("CODE", &req()));
        assert_register_request(&sent);
        let err = format!("{:#}", res.unwrap_err());
        assert!(
            err.starts_with("register: HTTP 400: InvalidValue: bad code"),
            "{err}"
        );
        assert!(!err.contains("Atna|planted"), "{err}");
    }

    #[test]
    fn register_http_non_json_is_status_only() {
        let (res, sent) = with_http(502, b"<html>Atna|planted</html>", || {
            register("CODE", &req())
        });
        assert_register_request(&sent);
        let err = res.unwrap_err();
        assert_eq!(err.to_string(), "register: HTTP 502");
        assert!(!format!("{err:#}").contains("Atna|planted"), "{err:#}");
    }

    #[test]
    fn refresh_http_success() {
        let id = parse_register(&register_fixture(), &req(), 0.0).unwrap();
        let body =
            serde_json::to_vec(&json!({"access_token": "Atna|new", "expires_in": 3600})).unwrap();
        let (next, sent) = with_http(200, &body, || refresh(&id));
        assert_refresh_request(&sent);
        let next = next.unwrap();
        assert_eq!(next.access_token, "Atna|new");
        assert_eq!(next.refresh_token, id.refresh_token);
    }

    #[test]
    fn refresh_http_error_body_surfaces_code_not_tokens() {
        let id = parse_register(&register_fixture(), &req(), 0.0).unwrap();
        let body = serde_json::to_vec(&json!({
            "error": "invalid_grant",
            "error_description": "expired",
            "access_token": "Atna|planted",
        }))
        .unwrap();
        let (res, sent) = with_http(400, &body, || refresh(&id));
        assert_refresh_request(&sent);
        let err = format!("{:#}", res.unwrap_err());
        assert_eq!(err, "refresh: HTTP 400: invalid_grant: expired");
    }

    #[test]
    fn refresh_http_non_json_is_status_only() {
        let id = parse_register(&register_fixture(), &req(), 0.0).unwrap();
        let (res, sent) = with_http(503, b"<html>Atna|planted</html>", || refresh(&id));
        assert_refresh_request(&sent);
        assert_eq!(format!("{:#}", res.unwrap_err()), "refresh: HTTP 503");
    }
}
