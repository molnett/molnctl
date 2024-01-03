use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use oauth2::{
    basic::BasicClient, reqwest::http_client, AuthUrl, ClientId, CsrfToken, TokenResponse, TokenUrl,
};
use tiny_http::{Response, Server};

use crate::config::user::Token;

use super::CommandBase;

#[derive(Parser)]
#[derive(Debug)]
#[command(author, version, about, long_about)]
pub struct Auth {}

impl Auth {
    pub fn execute(&self, base: &mut CommandBase) -> Result<()> {
        let server = Server::http("localhost:0").unwrap();
        let local_port = server.server_addr().to_ip().unwrap().port();
        let redirect_uri = format!("http://localhost:{}/oauth2/callback", local_port);

        let url = base.user_config().get_url();
        let client = BasicClient::new(
            ClientId::new("124a489e-93f7-4dd6-abae-1ed4c692bdc7".to_string()),
            None,
            AuthUrl::new(format!("{}/oauth2/auth", url)).unwrap(),
            Some(TokenUrl::new(format!("{}/oauth2/token", url)).unwrap()),
        )
        .set_redirect_uri(
            oauth2::RedirectUrl::new(redirect_uri.clone()).unwrap(),
        );

        let (pkce_code_challenge, pkce_verifier) = oauth2::PkceCodeChallenge::new_random_sha256();

        let (auth_url, _) = client
            .authorize_url(|| CsrfToken::new(redirect_uri)) // TODO: create state and verify it instead of using redirect uri
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
            // TODO: the api returns "expiry":"2024-01-01T11:03:53.485518152+01:00"
            if let Some(expires_in) = oauthtoken.expires_in() {
                token.expiry =
                    Some(Utc::now() + chrono::Duration::seconds(expires_in.as_secs() as i64));
            }

            base.user_config_mut().write_token(token)?;

            request.respond(Response::from_string("Success! You can close this tab now"))?;

            return Ok(());
        }
        Ok(())
    }
}
