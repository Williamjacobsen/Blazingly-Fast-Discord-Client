use reqwest::{
    blocking::Client,
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
};
use std::{env, error::Error};

use dotenv::dotenv;

pub fn initialize() -> Result<(), Box<dyn Error>> {
    dotenv()?;
    Ok(())
}

pub fn fetch_profile_information(user_id: &str) -> Result<(), Box<dyn Error>> {
    let authorization_token = env::var("DISCORD_TOKEN")?;
    let url = format!("https://discord.com/api/v9/users/{}/profile?with_mutual_guilds=false&with_mutual_friends=false&with_mutual_friends_count=false", user_id);

    let client = Client::new();

    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&authorization_token)?);

    let response = client.get(&url).headers(headers).send()?;

    if response.status().is_success() {
        let text = response.text()?;
        println!("{}", text);
    } else {
        println!("Request failed with status: {}", response.status());
    }

    Ok(())
}
