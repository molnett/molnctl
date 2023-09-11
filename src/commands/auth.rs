use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use oauth2::{
    basic::BasicClient, reqwest::http_client, AuthUrl, ClientId, CsrfToken, TokenResponse, TokenUrl,
};
use tiny_http::{Response, Server};

use crate::config::config::Token;

use super::CommandBase;

#[derive(Parser)]
#[command(author, version, about, long_about)]
pub struct Auth {}

impl Auth {
    pub fn execute(&self, base: &mut CommandBase) -> Result<()> {
        let server = Server::http("localhost:0").unwrap();

        let client = BasicClient::new(
            ClientId::new("60c03580-f2c1-4c67-bde8-57463c7d8c47".to_string()),
            None,
            AuthUrl::new("http://localhost:8000/oauth2/auth".to_string()).unwrap(),
            Some(TokenUrl::new("http://localhost:8000/oauth2/token".to_string()).unwrap()),
        )
        .set_redirect_uri(
            oauth2::RedirectUrl::new("http://localhost:8000/oauth2/callback".to_string()).unwrap(),
        );

        let (pkce_code_challenge, pkce_verifier) = oauth2::PkceCodeChallenge::new_random_sha256();

        let state = format!(
            "http://localhost:{}/oauth/callback",
            server.server_addr().to_ip().unwrap().port()
        );

        let (auth_url, _) = client
            .authorize_url(|| CsrfToken::new(state.clone()))
            .set_pkce_challenge(pkce_code_challenge)
            .url();

        println!("Browse to: {}", auth_url);

        println!("Listening on {}", server.server_addr());
        for request in server.incoming_requests() {
            let url = request.url();

            let code = url.split("?code=").collect::<Vec<&str>>()[1];

            let oauthtoken = client
                .exchange_code(oauth2::AuthorizationCode::new(code.to_string()))
                .set_pkce_verifier(pkce_verifier)
                .request(http_client)
                .unwrap();

            let mut token = Token::new();

            token.access_token = oauthtoken.access_token().secret().to_string();
            if let Some(refresh_token) = oauthtoken.refresh_token() {
                token.refresh_token = Some(refresh_token.secret().to_string());
            }
            if let Some(expires_in) = oauthtoken.expires_in() {
                token.expiry =
                    Some(Utc::now() + chrono::Duration::seconds(expires_in.as_secs() as i64));
            }

            base.user_config_mut()?.set_token(token)?;

            request.respond(Response::from_string("Success!"))?;

            return Ok(());
        }
        Ok(())
    }
}
