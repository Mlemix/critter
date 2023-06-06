#![allow(unused)]

use reqwest::multipart::Form;
use reqwest::{Method, Url};
use reqwest::{header::AUTHORIZATION, Client};
use serde::de::DeserializeOwned;

use std::{env, fs, time::{SystemTime, UNIX_EPOCH}, collections::HashMap, str::FromStr};

use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
struct TwitterApiResponseError {
    code: u32,
    message: String
}

#[derive(Debug, Deserialize)]
pub struct TwitterPostData {
    pub id: String,
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TwitterPost {
    data: Option<TwitterPostData>,
    errors: Option<Vec<TwitterApiResponseError>>
}

pub mod auth;
use auth::*;

pub struct TweetMediaBuilder(pub HashMap<&'static str, Value>);
impl TweetMediaBuilder {
    pub fn add(&mut self, media: Option<TwitterMediaResponse>) -> &mut Self {
        if let Some(data) = media {
            let medias = self
                .0
                .entry("media_ids")
                .or_insert_with(|| Value::from(Vec::<Value>::new()))
                .as_array_mut()
                .expect("no media_ids???");

            medias.push(Value::from(data.media_id_string.as_str()));
        }
        
        self
    }

    pub fn id(&mut self, id: u64) -> &mut Self {
        let medias = self
            .0
            .entry("media_ids")
            .or_insert_with(|| Value::from(Vec::<Value>::new()))
            .as_array_mut()
            .expect("no media_ids???");

        medias.push(Value::String(id.to_string()));
        self
    }
}
impl Default for TweetMediaBuilder {
    fn default() -> TweetMediaBuilder {
        let mut map = HashMap::new();
        map.insert("media_ids", Value::from(Vec::<Value>::new()));

        TweetMediaBuilder(map)
    }
}

#[derive(Default)]
pub struct TweetBuilder(pub HashMap<&'static str, Value>);
impl TweetBuilder {
    pub fn text(&mut self, text: &str) -> &mut Self {
        self.0.insert("text", Value::from(text));
        self
    }

    /*pub fn add_media(&mut self, media: &str) -> &mut Self {
        self.0.insert(
            "media",
            json!({"media_ids": [media]}),
        );
        self
    }*/

    pub fn media<F>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce(&mut TweetMediaBuilder) -> &mut TweetMediaBuilder,
    {
        let mut media = TweetMediaBuilder::default();
        f(&mut media);

        self.0.insert("media", json!(media.0));
        self
    }
}

pub mod error;
use error::Error;

#[derive(Debug, Deserialize)]
struct TwitterApiResponse {
    detail: Option<String>,
    errors: Option<Vec<TwitterApiResponseError>>,
    data: Option<Value>
}

pub struct TwitterClient {
    http: Client,
    auth: TwitterAuth
}
impl TwitterClient {
    pub fn new(auth: TwitterAuth) -> Result<Self, Box<dyn std::error::Error>> {
        let http = Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.4 Safari/605.1.15")
            .build()?;

        Ok(Self {
            http,
            auth
        })
    }

    async fn _request<T: DeserializeOwned>(&mut self, method: &str, url: &str, query: Option<&[(&str, &str)]>) -> Result<T, Error> {
        let res = self.http.request(
            Method::from_str(method).unwrap_or(Method::GET),
            Url::parse_with_params(url, query.unwrap_or_default()).unwrap()
        )
            .header(AUTHORIZATION, &self.auth.header(
                method,
                url,
                query
            ))
            .send()
            .await?
            .json::<TwitterApiResponse>()
            .await?;

        match res.data {
            Some(data) => Ok(serde_json::from_value(data).unwrap()),
            None => {
                if let Some(detail) = res.detail {
                    match detail.as_ref() {
                        "Too Many Requests" => Err(Error::TooManyRequests),
                        _ => Err(Error::Unknown)
                    }
                } else if let Some(errors) = res.errors {
                    println!("got errors: {:?}", errors);
                    Err(Error::Unknown)
                } else {
                    Err(Error::Unknown)
                }
            }
        }
    }

    async fn _json_request<T: DeserializeOwned>(&mut self, method: &str, url: &str, json: Value, query: Option<&[(&str, &str)]>) -> Result<T, Error> {
        let res = self.http.request(
            Method::from_str(method).unwrap_or(Method::GET),
            Url::parse_with_params(url, query.unwrap_or_default()).unwrap()
        )
            .header(AUTHORIZATION, &self.auth.header(
                method,
                url,
                query
            ))
            .json(&json)
            .send()
            .await?
            .json::<TwitterApiResponse>()
            .await?;

        match res.data {
            Some(data) => Ok(serde_json::from_value(data).unwrap()),
            None => {
                if let Some(detail) = res.detail {
                    match detail.as_ref() {
                        "Too Many Requests" => Err(Error::TooManyRequests),
                        _ => Err(Error::Unknown)
                    }
                } else if let Some(errors) = res.errors {
                    println!("got errors: {:?}", errors);
                    Err(Error::Unknown)
                } else {
                    Err(Error::Unknown)
                }
            }
        }
    }

    async fn _multipart_request<T: DeserializeOwned>(&mut self, method: &str, url: &str, multipart: Form) -> Result<T, Error> {
        let res = self.http.request(
            Method::from_str(method).unwrap_or(Method::GET),
            url
        )
            .header(AUTHORIZATION, &self.auth.header(
                method,
                url,
                None
            ))
            .multipart(multipart)
            .send()
            .await?
            .json::<Value>()
            .await?;

        if let Some(errors) = res.get("errors") {
            println!("got errors: {:?}", errors);
            return Err(Error::Unknown)
        }

        match serde_json::from_value::<T>(res) {
            Ok(data) => Ok(data),
            Err(_) => Err(Error::BadMedia)
        }
    }

    pub async fn me(&mut self, fields: Option<&[&str]>) -> Result<TwitterUserData, Error> {
        let fields_str = fields.map_or(String::new(), |f| f.join(","));
        let query = [("user.fields", fields_str.as_str())];
    
        self._request(
            "GET",
            "https://api.twitter.com/2/users/me",
            Some(&query)
        ).await
    }

    pub async fn upload_media(&mut self, path: &str, filename: Option<String>) -> Result<TwitterMediaResponse, Error> {
        let file_bytes;
        let mime;
        if path.starts_with("http") {
            let res = reqwest::get(path)
                .await;

            match res {
                Ok(data) => {
                    file_bytes = data.bytes().await?.to_vec();
                    mime = "image/jpg"
                },
                _ => return Err(Error::BadMedia) 
            }
        } else {
            match fs::read(path) {
                Ok(bytes) => {
                    file_bytes = bytes;
                    mime = infer::get(&file_bytes).unwrap().mime_type();
                },
                _ => return Err(Error::BadMedia)
            }
        }

        if file_bytes.len() <= 1024 * 1024 { // simple upload
            let file_part = reqwest::multipart::Part::bytes(file_bytes)
                .file_name(filename.unwrap_or("media".into()))
                .mime_str(mime)
                .unwrap();
            let form = reqwest::multipart::Form::new()
                .part("media", file_part);

            self._multipart_request(
                "POST",
                "https://upload.twitter.com/1.1/media/upload.json",
                form
            ).await
        } else {
            Err(Error::NoUserData)
        }
    }

    pub async fn tweet<F>(&mut self, f: F) -> Result<TwitterPostData, Error>
    where
        F: FnOnce(&mut TweetBuilder) -> &mut TweetBuilder,
    {
        let mut tweet = TweetBuilder::default();
        f(&mut tweet);

        self._json_request(
            "POST",
            "https://api.twitter.com/2/tweets",
            json!(tweet.0),
            None
        ).await
    }
}

#[derive(Debug, Deserialize)]
pub struct TwitterMediaResponse {
    media_id: u64,
    expires_after_secs: u32,
    media_id_string: String
}
impl TwitterMediaResponse {
    pub fn id(&mut self) -> &str {
        &self.media_id_string
    }
}

#[derive(Debug, Deserialize)]
pub struct TwitterUserData {
    pub id: String,
    pub name: String,
    pub username: String,
    pub description: Option<String>,
    pub created_at: Option<String>
}

#[derive(Debug, Deserialize)]
struct TwitterUserResponse {
    detail: Option<String>,
    data: Option<TwitterUserData>
}