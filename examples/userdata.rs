use critter::{ TwitterClient, auth::TwitterAuth };
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let auth = TwitterAuth::from_oa1uc(
        &env::var("CONSUMER_KEY").unwrap(),
        &env::var("CONSUMER_SECRET").unwrap(),
        &env::var("ACCESS_TOKEN").unwrap(),
        &env::var("ACCESS_TOKEN_SECRET").unwrap()
    );

    let mut twitter = TwitterClient::new(auth)?;

    // Print all the default fields
    match twitter.me(None).await {
        Ok(data) => println!("My name is {} and my username is {}. Also, my id is {}.", data.name(), data.username(), data.id()),
        Err(e) => println!("Error: {}", e) // Can be something like ratelimit
    }

    // Request the description
    match twitter.me(Some(&["description"])).await {
        Ok(data) => println!("My description is \"{}\"", data.description()),
        Err(e) => println!("Error: {}", e)
    }

    // Request the date your account was created at
    match twitter.me(Some(&["created_at"])).await {
        Ok(data) => println!("I made my account on {}", data.created_at()),
        Err(e) => println!("Error: {}", e)
    }

    // Request multiple fields
    match twitter.me(Some(&["description", "created_at"])).await {
        Ok(data) => println!("My description is \"{}\" and I made my account on {}", data.description(), data.created_at()),
        Err(e) => println!("Error: {}", e)
    }
    
    Ok(())
}