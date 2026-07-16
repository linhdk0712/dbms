use anyhow::{Context, Result};
use openidconnect::{
    core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata},
    reqwest::async_http_client,
    AccessTokenHash, AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    OAuth2TokenResponse, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
};
use url::Url;

use crate::config::Config;

/// Thông tin user trả về sau khi xác thực OIDC thành công.
#[derive(Debug, Clone)]
pub struct OidcUserInfo {
    /// Subject claim — unique identifier từ OIDC provider
    pub subject: String,
    pub email: String,
    pub display_name: String,
}

/// Dữ liệu cần lưu vào session khi bắt đầu OIDC flow.
/// Phải truyền lại khi xử lý callback.
///
/// Note: PkceCodeVerifier không impl Clone — OidcSession chỉ dùng một lần rồi consume.
#[derive(Debug)]
pub struct OidcSession {
    pub csrf_token: CsrfToken,
    pub nonce: Nonce,
    pub pkce_verifier: PkceCodeVerifier,
}

/// Khởi tạo OIDC CoreClient bằng cách auto-discover provider metadata.
/// Gọi một lần lúc startup, lưu vào AppState.
pub async fn create_oidc_client(config: &Config) -> Result<CoreClient> {
    let issuer_url =
        IssuerUrl::new(config.oidc_issuer_url.clone()).context("invalid OIDC issuer URL")?;

    let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, async_http_client)
        .await
        .context("failed to discover OIDC provider metadata")?;

    let client = CoreClient::from_provider_metadata(
        provider_metadata,
        ClientId::new(config.oidc_client_id.clone()),
        Some(ClientSecret::new(config.oidc_client_secret.clone())),
    )
    .set_redirect_uri(
        RedirectUrl::new(config.oidc_redirect_uri.clone())
            .context("invalid OIDC redirect URI")?,
    );

    Ok(client)
}

/// Tạo authorization URL để redirect user đến OIDC provider.
/// Trả về URL, CSRF token, nonce, và PKCE verifier — tất cả phải lưu vào session.
pub fn build_auth_url(client: &CoreClient) -> (Url, OidcSession) {
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, csrf_token, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .add_extra_param("prompt", "login")
        .set_pkce_challenge(pkce_challenge)
        .url();

    let session = OidcSession {
        csrf_token,
        nonce,
        pkce_verifier,
    };

    (auth_url, session)
}

/// Xử lý OIDC callback: exchange code lấy token, verify claims, trả về user info.
///
/// # Security
/// - Verify CSRF state trước tiên
/// - Verify PKCE
/// - Verify nonce trong ID token
/// - Verify access_token hash (chống substitution attack)
pub async fn handle_oidc_callback(
    client: &CoreClient,
    code: String,
    state_from_query: &str,
    session: OidcSession,
) -> Result<OidcUserInfo> {
    // 1. Verify CSRF state — bắt buộc trước mọi xử lý khác
    if session.csrf_token.secret() != state_from_query {
        anyhow::bail!("CSRF state mismatch: possible CSRF attack");
    }

    // 2. Exchange authorization code lấy token (consume pkce_verifier)
    let token_response = client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(session.pkce_verifier)
        .request_async(async_http_client)
        .await
        .context("failed to exchange authorization code")?;

    // 3. Lấy và verify ID token
    let id_token = token_response
        .id_token()
        .context("OIDC provider did not return an ID token")?;

    let verifier = client.id_token_verifier();
    let claims = id_token
        .claims(&verifier, &session.nonce)
        .context("failed to verify ID token claims")?;

    // 4. Verify access_token hash — chống substitution attack
    if let Some(expected_hash) = claims.access_token_hash() {
        let signing_alg = id_token
            .signing_alg()
            .context("failed to get ID token signing algorithm")?;

        let actual_hash = AccessTokenHash::from_token(
            token_response.access_token(),
            &signing_alg,
        )
        .context("failed to compute access token hash")?;

        if &actual_hash != expected_hash {
            anyhow::bail!("access token hash mismatch: possible token substitution attack");
        }
    }

    // 5. Extract user info từ claims
    let subject = claims.subject().to_string();

    let email = claims
        .email()
        .map(|e| e.to_string())
        .unwrap_or_else(|| format!("{subject}@unknown"));

    let display_name = claims
        .name()
        .and_then(|n| n.get(None))
        .map(|n| n.to_string())
        .or_else(|| claims.preferred_username().map(|u| u.to_string()))
        .unwrap_or_else(|| email.split('@').next().unwrap_or(&subject).to_string());

    Ok(OidcUserInfo {
        subject,
        email,
        display_name,
    })
}
