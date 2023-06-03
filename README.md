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
### Creating a Client - OAuth 1.0a User Context (With Provided OAuth Tokens)
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let auth = TwitterAuth::from_3pin(
        &env::var("CONSUMER_KEY").unwrap(),
        &env::var("CONSUMER_SECRET").unwrap(),
        &env::var("ACCESS_TOKEN").unwrap(),
        &env::var("ACCESS_TOKEN_SECRET").unwrap()
    );

    let mut twitter = TwitterClient::new(auth)?;

    Ok(())
}
```

### Getting the Details of The Authenticated User
```rust
match twitter.me(None).await {
    Ok(data) => println!("My name is {}", data.name),
    Err(e) => println!("Error: {}", e) // Can be something like ratelimit
}
```
An example of obtaining additional details such as `description` and `created_at` is provided [here](https://github.com/Mlemix/critter).
