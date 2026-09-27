use axum::body::Bytes;
use axum::extract::{FromRequest, Request};
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};

/// クエリとフォーム送信（OpenSubsonic の formPost）の引数をまとめたもの。
/// `id=1&id=2` のような繰り返しがあるので、順序付きの組で持つ。
#[derive(Debug, Default, Clone)]
pub struct Params(Vec<(String, String)>);

/// ログに値を残さない引数。
const SECRET_KEYS: [&str; 4] = ["p", "t", "s", "apiKey"];

impl Params {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// 繰り返された引数の値を、送られた順に返す。
    pub fn get_all<'a>(&'a self, key: &'a str) -> impl Iterator<Item = &'a str> {
        self.0
            .iter()
            .filter(move |(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    pub fn contains(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// 認証情報を伏せた、ログ用の文字列。
    pub fn masked(&self) -> String {
        let mut out = form_urlencoded::Serializer::new(String::new());
        for (k, v) in &self.0 {
            let v = if SECRET_KEYS.contains(&k.as_str()) {
                "***"
            } else {
                v
            };
            out.append_pair(k, v);
        }
        out.finish()
    }
}

impl<S: Send + Sync> FromRequest<S> for Params {
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let mut pairs: Vec<(String, String)> = req
            .uri()
            .query()
            .map(|q| form_urlencoded::parse(q.as_bytes()).into_owned().collect())
            .unwrap_or_default();
        let is_form = req
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|ct| ct.starts_with("application/x-www-form-urlencoded"));
        if is_form {
            let body = Bytes::from_request(req, state)
                .await
                .map_err(IntoResponse::into_response)?;
            pairs.extend(form_urlencoded::parse(&body).into_owned());
        }
        Ok(Self(pairs))
    }
}

impl<K: Into<String>, V: Into<String>> FromIterator<(K, V)> for Params {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self(
            iter.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masked_hides_secrets() {
        let params: Params = [("u", "a"), ("t", "x"), ("s", "y"), ("c", "Symfonium")]
            .into_iter()
            .collect();
        assert_eq!(params.masked(), "u=a&t=***&s=***&c=Symfonium");
    }
}
