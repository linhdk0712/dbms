use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::models::user::Role;

/// JWT Claims cho SECS.
/// Có cả `sub: String` (JWT standard) VÀ `user_id: Uuid` (type-safe cho DB queries).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    // ── Standard JWT fields ────────────────────────────────────────────
    /// Subject — user_id dưới dạng String (JWT RFC 7519)
    pub sub: String,
    /// Expiration timestamp (Unix seconds) — tối đa 8 giờ
    pub exp: usize,
    /// Issued at timestamp (Unix seconds)
    pub iat: usize,
    /// Issuer — "secs-backend"
    pub iss: String,

    // ── Custom SECS claims ─────────────────────────────────────────────
    /// User ID dưới dạng Uuid — tiện dùng trong DB queries mà không cần parse
    pub user_id: Uuid,
    pub email: String,
    pub display_name: String,
    /// Danh sách roles — ["MAKER", "CHECKER"]
    pub roles: Vec<Role>,
}

impl Claims {
    /// Kiểm tra user có role cụ thể không.
    pub fn has_role(&self, role: &Role) -> bool {
        self.roles.contains(role)
    }

    /// Kiểm tra user có ít nhất một trong các roles không.
    pub fn has_any_role(&self, roles: &[Role]) -> bool {
        roles.iter().any(|r| self.roles.contains(r))
    }

    /// True nếu user là Admin.
    pub fn is_admin(&self) -> bool {
        self.has_role(&Role::Admin)
    }
}

const ISSUER: &str = "secs-backend";
/// Thời hạn token: 8 giờ (theo security rules)
const TOKEN_EXPIRY_SECS: u64 = 8 * 60 * 60;

/// Tạo JWT token mới.
/// Token hết hạn sau 8 giờ — theo security rules.
pub fn generate_token(
    user_id: Uuid,
    email: &str,
    display_name: &str,
    roles: Vec<Role>,
    secret: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before Unix epoch")
        .as_secs();

    let claims = Claims {
        sub: user_id.to_string(),
        exp: (now + TOKEN_EXPIRY_SECS) as usize,
        iat: now as usize,
        iss: ISSUER.to_string(),
        user_id,
        email: email.to_string(),
        display_name: display_name.to_string(),
        roles,
    };

    let header = Header::new(Algorithm::HS256);
    let key = EncodingKey::from_secret(secret.as_bytes());
    encode(&header, &claims, &key)
}

/// Xác thực và decode JWT token.
/// Tự động reject nếu: hết hạn, sai signature, sai issuer.
pub fn verify_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let key = DecodingKey::from_secret(secret.as_bytes());

    let mut validation = Validation::new(Algorithm::HS256);
    // validate_exp = true theo mặc định — reject token hết hạn
    validation.validate_exp = true;
    validation.set_issuer(&[ISSUER]);

    let data = decode::<Claims>(token, &key, &validation)?;
    Ok(data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;

    const TEST_SECRET: &str = "test-secret-at-least-32-characters-long!!";
    const SHORT_SECRET: &str = "short";

    fn test_user_id() -> Uuid {
        Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
    }

    fn make_token_with_secret(secret: &str) -> String {
        generate_token(
            test_user_id(),
            "test@example.com",
            "Test User",
            vec![Role::Maker],
            secret,
        )
        .expect("token generation should succeed")
    }

    // ── Task 7: Tests cho encode/decode ────────────────────────────────

    #[test]
    fn test_generate_and_verify_token() {
        let token = make_token_with_secret(TEST_SECRET);
        let claims = verify_token(&token, TEST_SECRET).expect("token should be valid");

        assert_eq!(claims.user_id, test_user_id());
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.display_name, "Test User");
        assert_eq!(claims.roles, vec![Role::Maker]);
        assert_eq!(claims.iss, ISSUER);
        assert_eq!(claims.sub, test_user_id().to_string());
    }

    #[test]
    fn test_verify_wrong_secret_fails() {
        let token = make_token_with_secret(TEST_SECRET);
        let result = verify_token(&token, "wrong-secret-that-is-at-least-32-chars!!!");
        assert!(result.is_err(), "verification with wrong secret must fail");
    }

    #[test]
    fn test_verify_tampered_token_fails() {
        let token = make_token_with_secret(TEST_SECRET);
        // Thêm ký tự vào cuối token để tamper
        let tampered = format!("{token}x");
        let result = verify_token(&tampered, TEST_SECRET);
        assert!(result.is_err(), "tampered token must fail verification");
    }

    #[test]
    fn test_expired_token_fails() {
        // Tạo token với exp trong quá khứ bằng cách encode thủ công
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let expired_claims = Claims {
            sub: test_user_id().to_string(),
            exp: now - 120, // đã hết hạn 2 phút trước (vượt qua leeway mặc định 60s)
            iat: now - 3600,
            iss: ISSUER.to_string(),
            user_id: test_user_id(),
            email: "test@example.com".to_string(),
            display_name: "Test User".to_string(),
            roles: vec![Role::Maker],
        };

        let key = EncodingKey::from_secret(TEST_SECRET.as_bytes());
        let expired_token =
            encode(&Header::new(Algorithm::HS256), &expired_claims, &key).unwrap();

        let result = verify_token(&expired_token, TEST_SECRET);
        assert!(result.is_err(), "expired token must be rejected");
    }

    #[test]
    fn test_has_role() {
        let token = make_token_with_secret(TEST_SECRET);
        let claims = verify_token(&token, TEST_SECRET).unwrap();

        assert!(claims.has_role(&Role::Maker));
        assert!(!claims.has_role(&Role::Checker));
        assert!(!claims.has_role(&Role::Admin));
    }

    #[test]
    fn test_has_any_role() {
        let token = generate_token(
            test_user_id(),
            "dual@example.com",
            "Dual Role User",
            vec![Role::Maker, Role::Checker],
            TEST_SECRET,
        )
        .unwrap();
        let claims = verify_token(&token, TEST_SECRET).unwrap();

        assert!(claims.has_any_role(&[Role::Checker, Role::Admin]));
        assert!(!claims.has_any_role(&[Role::Admin]));
    }

    #[test]
    fn test_multiple_roles_preserved() {
        let roles = vec![Role::Maker, Role::Checker, Role::Admin];
        let token = generate_token(
            test_user_id(),
            "admin@example.com",
            "Admin User",
            roles.clone(),
            TEST_SECRET,
        )
        .unwrap();

        let claims = verify_token(&token, TEST_SECRET).unwrap();
        assert_eq!(claims.roles, roles);
        assert!(claims.is_admin());
    }

    #[test]
    fn test_token_has_correct_issuer() {
        let token = make_token_with_secret(TEST_SECRET);
        let claims = verify_token(&token, TEST_SECRET).unwrap();
        assert_eq!(claims.iss, "secs-backend");
    }

    #[test]
    fn test_token_expiry_within_8_hours() {
        let token = make_token_with_secret(TEST_SECRET);
        let claims = verify_token(&token, TEST_SECRET).unwrap();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        assert!(claims.exp > now, "token must not be expired immediately");
        assert!(
            claims.exp <= now + (TOKEN_EXPIRY_SECS as usize) + 5, // +5s tolerance
            "token expiry must not exceed 8 hours"
        );
    }

    #[test]
    fn test_short_secret_still_works() {
        // jsonwebtoken cho phép secret ngắn, chỉ config.rs mới enforce min length
        let token = make_token_with_secret(SHORT_SECRET);
        let claims = verify_token(&token, SHORT_SECRET).unwrap();
        assert_eq!(claims.email, "test@example.com");
    }
}
