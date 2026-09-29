use md5::{Digest, Md5};

use super::{Error, ErrorCode, Params};
use crate::Credentials;

/// Subsonic の認証の引数。
const AUTH_KEYS: [&str; 5] = ["u", "p", "t", "s", "apiKey"];

/// 認証の引数を一つでも持つか。持たなければ Cookie のセッションで認証する（docs/server.md）。
pub fn has_credentials(params: &Params) -> bool {
    AUTH_KEYS.iter().any(|k| params.contains(k))
}

/// 利用者名とパスワードが正しいか。
pub fn verify_password(creds: &Credentials, user: &str, password: &str) -> bool {
    // 利用者名が違っても、パスワードの比較を省かない
    let password_ok = constant_time_eq(password.as_bytes(), creds.password.as_bytes());
    constant_time_eq(user.as_bytes(), creds.user.as_bytes()) & password_ok
}

/// Subsonic の認証（`p`、`t` と `s`、OpenSubsonic の `apiKey`）を確かめる。
pub fn authenticate(params: &Params, creds: &Credentials) -> Result<(), Error> {
    if let Some(api_key) = params.get("apiKey") {
        return authenticate_api_key(params, api_key, creds);
    }

    let user = params.get("u").ok_or_else(|| {
        Error::new(
            ErrorCode::MissingParameter,
            "required parameter is missing: u",
        )
    })?;
    let password = params.get("p");
    let token = params.get("t");
    let salt = params.get("s");

    let valid = match (password, token, salt) {
        (Some(_), Some(_), _) | (Some(_), _, Some(_)) => {
            return Err(Error::new(
                ErrorCode::ConflictingAuth,
                "p cannot be combined with t or s",
            ));
        }
        (Some(p), None, None) => {
            let p = decode_password(p).ok_or_else(|| {
                Error::new(ErrorCode::WrongCredentials, "wrong username or password")
            })?;
            constant_time_eq(p.as_bytes(), creds.password.as_bytes())
        }
        (None, Some(t), Some(s)) => {
            let expected = hex::encode(Md5::digest(format!("{}{s}", creds.password)));
            constant_time_eq(t.to_ascii_lowercase().as_bytes(), expected.as_bytes())
        }
        _ => {
            return Err(Error::new(
                ErrorCode::MissingParameter,
                "required parameter is missing: p, or t and s",
            ));
        }
    };

    if valid && constant_time_eq(user.as_bytes(), creds.user.as_bytes()) {
        Ok(())
    } else {
        Err(Error::new(
            ErrorCode::WrongCredentials,
            "wrong username or password",
        ))
    }
}

fn authenticate_api_key(params: &Params, api_key: &str, creds: &Credentials) -> Result<(), Error> {
    if ["u", "p", "t", "s"].iter().any(|k| params.contains(k)) {
        return Err(Error::new(
            ErrorCode::ConflictingAuth,
            "apiKey cannot be combined with u, p, t or s",
        ));
    }
    let Some(expected) = &creds.api_key else {
        return Err(Error::new(
            ErrorCode::AuthMechanismNotSupported,
            "apiKey authentication is not enabled",
        ));
    };
    if constant_time_eq(api_key.as_bytes(), expected.as_bytes()) {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::InvalidApiKey, "invalid API key"))
    }
}

/// `enc:` で始まるパスワードは 16 進で符号化されている。
fn decode_password(p: &str) -> Option<String> {
    match p.strip_prefix("enc:") {
        Some(encoded) => String::from_utf8(hex::decode(encoded).ok()?).ok(),
        None => Some(p.to_owned()),
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn creds(api_key: Option<&str>) -> Credentials {
        Credentials {
            user: "inaba".into(),
            password: "sesame".into(),
            api_key: api_key.map(Into::into),
        }
    }

    fn check(pairs: &[(&str, &str)], api_key: Option<&str>) -> Result<(), ErrorCode> {
        let params: Params = pairs.iter().copied().collect();
        authenticate(&params, &creds(api_key)).map_err(|e| e.code)
    }

    #[test]
    fn plain_and_hex_password() {
        assert_eq!(check(&[("u", "inaba"), ("p", "sesame")], None), Ok(()));
        // "sesame" を 16 進にしたもの
        assert_eq!(
            check(&[("u", "inaba"), ("p", "enc:736573616d65")], None),
            Ok(())
        );
        assert_eq!(
            check(&[("u", "inaba"), ("p", "wrong")], None),
            Err(ErrorCode::WrongCredentials)
        );
    }

    #[test]
    fn token() {
        let t = hex::encode(Md5::digest("sesamesalt"));
        assert_eq!(
            check(&[("u", "inaba"), ("t", &t), ("s", "salt")], None),
            Ok(())
        );
        assert_eq!(
            check(&[("u", "inaba"), ("t", &t), ("s", "other")], None),
            Err(ErrorCode::WrongCredentials)
        );
        assert_eq!(
            check(&[("u", "someone"), ("t", &t), ("s", "salt")], None),
            Err(ErrorCode::WrongCredentials)
        );
    }

    #[test]
    fn missing_or_conflicting() {
        assert_eq!(check(&[], None), Err(ErrorCode::MissingParameter));
        assert_eq!(
            check(&[("u", "inaba"), ("t", "x")], None),
            Err(ErrorCode::MissingParameter)
        );
        assert_eq!(
            check(&[("u", "inaba"), ("p", "sesame"), ("t", "x")], None),
            Err(ErrorCode::ConflictingAuth)
        );
    }

    #[test]
    fn api_key() {
        assert_eq!(check(&[("apiKey", "k")], Some("k")), Ok(()));
        assert_eq!(
            check(&[("apiKey", "x")], Some("k")),
            Err(ErrorCode::InvalidApiKey)
        );
        assert_eq!(
            check(&[("apiKey", "k")], None),
            Err(ErrorCode::AuthMechanismNotSupported)
        );
        assert_eq!(
            check(&[("apiKey", "k"), ("u", "inaba")], Some("k")),
            Err(ErrorCode::ConflictingAuth)
        );
    }
}
