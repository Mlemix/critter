# Critter
Simple rust library to interact with the twitter V2 api.

## Getting Started
Before proceeding, ensure you have your Twitter Developer App credentials handy - Consumer Key, Consumer Secret, Access Token, and Access Token Secret.

## Installation
Include the following in your `Cargo.toml`:
```toml
[dependencies]
critter = "0.1.0"
```

## Basic Examples
### Creating a Client - OAuth1.0a
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut twitter = TwitterClient::new(
        &env::var("CONSUMER_KEY").unwrap(), // Your consumer key
        &env::var("CONSUMER_SECRET").unwrap(), // Your consumer secret
        &env::var("ACCESS_TOKEN").unwrap(), // Your access token
        &env::var("ACCESS_TOKEN_SECRET").unwrap() // Your access token secret
    )?;

    Ok(())
}
```

### Getting the Details of The Authenticated User
```rust
match twitter.me(None).await {
    Ok(data) => println!("My name: {}", data.name),
    Err(e) => println!("Error occurred: {}", e)
}
```
An example of obtaining additional details such as `description` and `created_at` is provided [here](https://github.com/Mlemix/critter).
