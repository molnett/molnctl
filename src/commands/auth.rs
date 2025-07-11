use std::{
    io::Write,
    process::{Command, Stdio},
};

use anyhow::{anyhow, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use oauth2::{
    basic::BasicClient, reqwest::http_client, AuthUrl, ClientId, CsrfToken, TokenResponse, TokenUrl,
};
use tiny_http::{Response, Server};

use crate::config::user::Token;

use super::CommandBase;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    long_about,
    subcommand_required = true,
    arg_required_else_help = true
)]
pub struct Auth {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

impl Auth {
    pub fn execute(self, base: CommandBase) -> Result<()> {
        match self.command {
            Some(Commands::Login(login)) => login.execute(base),
            Some(Commands::Docker(docker)) => docker.execute(base),
            None => Ok(()),
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Login to Molnett
    Login(Login),

    /// Login to Docker Registry using Molnett token
    Docker(Docker),
}

#[derive(Parser, Debug)]
pub struct Login {}

impl Login {
    pub fn execute(self, mut base: CommandBase) -> Result<()> {
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
        .set_redirect_uri(oauth2::RedirectUrl::new(redirect_uri.clone()).unwrap());

        let (pkce_code_challenge, pkce_verifier) = oauth2::PkceCodeChallenge::new_random_sha256();

        let (auth_url, _) = client
            .authorize_url(|| CsrfToken::new(redirect_uri)) // TODO: create state and verify it instead of using redirect uri
            .set_pkce_challenge(pkce_code_challenge)
            .url();

        println!("Browse to: {}", auth_url);

        println!("Listening on {}", server.server_addr());
        let request = server
            .incoming_requests()
            .next()
            .expect("server shutting down");
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
        } else {
            token.expiry = Some(Utc::now() + chrono::Duration::hours(1));
        }

        base.user_config_mut().write_token(token)?;

        request.respond(Response::from_string("Success! You can close this tab now"))?;

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct Docker {
    /// Docker registry URL
    #[arg(long, default_value = "oci.se-ume.mltt.art", hide = true)]
    pub registry: String,
}

impl Docker {
    pub fn execute(self, base: CommandBase) -> Result<()> {
        let token = base.user_config.get_token().ok_or_else(|| {
            anyhow!("Could not get Molnett token. Please run molnctl auth login.")
        })?;

        if base.user_config.is_token_expired() {
            println!("Token expired. Please run molnctl auth login.");
            return Ok(());
        }

        let mut command = Command::new("docker")
            .arg("login")
            .arg(&self.registry)
            .arg("--username=x")
            .arg("--password-stdin")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        if let Some(mut stdin) = command.stdin.take() {
            stdin.write_all(token.as_bytes())?;
        }

        let output = command.wait_with_output()?;

        if !output.status.success() {
            println!("{}", String::from_utf8_lossy(&output.stderr));
        } else {
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }
        Ok(())
    }
}
