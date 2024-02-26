#![allow(unused)]

use reqwest::multipart::Form;
use reqwest::{header::AUTHORIZATION, Client};
use reqwest::{Method, Url};
use serde::de::DeserializeOwned;

use std::{
    collections::HashMap,
    env, fs,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
struct TwitterApiResponseError {
    code: Option<u32>,
    message: String,
}

#[derive(Debug, Deserialize)]
pub struct TwitterPostData {
    id: String,
    text: Option<String>,
}
impl TwitterPostData {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn has_text(&self) -> bool {
        self.text.is_some()
    }
    pub fn description(&self) -> &str {
        match &self.text {
            Some(description) => description,
            None => "none",
        }
    }
}

#[derive(Debug, Deserialize)]
struct TwitterPost {
    data: Option<TwitterPostData>,
    errors: Option<Vec<TwitterApiResponseError>>,
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
use error::Error::ApiError;

#[derive(Debug, Deserialize)]
struct TwitterApiResponse {
    detail: Option<String>,
    errors: Option<Vec<TwitterApiResponseError>>,
    data: Option<Value>,
}

#[derive(Clone)]
pub struct TwitterClient {
    http: Client,
    auth: TwitterAuth,
}
impl TwitterClient {
    pub fn new(auth: TwitterAuth) -> Result<Self, Box<dyn std::error::Error>> {
        let http = Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.4 Safari/605.1.15")
            .build()?;

        Ok(Self { http, auth })
    }

    async fn _request_t<T: DeserializeOwned>(
        &mut self,
        method: &str,
        url: &str,
        query: Option<&[(&str, &str)]>,
    ) -> Result<T, Error> {
        Ok(self
            .http
            .request(
                Method::from_str(method).unwrap_or(Method::GET),
                Url::parse_with_params(url, query.unwrap_or_default()).unwrap(),
            )
            .header(AUTHORIZATION, &self.auth.header(method, url, query))
            .send()
            .await?
            .json::<T>()
            .await?)
    }

    fn collect_errors(&self, response: &TwitterApiResponse) -> Vec<String> {
        let mut res = vec![];
        if let Some(errors) = &response.errors {
            errors.iter().
                for_each(|e| res.push(e.message.to_string()));
        }
        if let Some(detail) = &response.detail {
            res.push(detail.to_string());
        }
        res
    }

    async fn _request<T: DeserializeOwned>(
        &mut self,
        method: &str,
        url: &str,
        query: Option<&[(&str, &str)]>,
    ) -> Result<T, Error> {
        let res = self
            .http
            .request(
                Method::from_str(method).unwrap_or(Method::GET),
                Url::parse_with_params(url, query.unwrap_or_default()).unwrap(),
            )
            .header(AUTHORIZATION, &self.auth.header(method, url, query))
            .send()
            .await?
            .json::<TwitterApiResponse>()
            .await?;

        match res.data {
            Some(data) => Ok(serde_json::from_value(data).unwrap()),
            None => {
                let error_strings = self.collect_errors(&res);
                if error_strings.contains(&String::from("Too Many Requests")) {
                    return Err(Error::TooManyRequests);
                }
                if !error_strings.is_empty() {
                    return Err(ApiError(error_strings.join(" ").to_string()));
                }
                return Err(Error::Unknown);
            }
        }
    }

    async fn _json_request<T: DeserializeOwned>(
        &mut self,
        method: &str,
        url: &str,
        json: Value,
        query: Option<&[(&str, &str)]>,
    ) -> Result<T, Error> {
        let res = self
            .http
            .request(
                Method::from_str(method).unwrap_or(Method::GET),
                Url::parse_with_params(url, query.unwrap_or_default()).unwrap(),
            )
            .header(AUTHORIZATION, &self.auth.header(method, url, query))
            .json(&json)
            .send()
            .await?
            .json::<TwitterApiResponse>()
            .await?;

        match res.data {
            Some(data) => Ok(serde_json::from_value(data).unwrap()),
            None => {
                let error_strings = self.collect_errors(&res);
                if error_strings.contains(&String::from("Too Many Requests")) {
                    return Err(Error::TooManyRequests);
                }
                if !error_strings.is_empty() {
                    return Err(ApiError(error_strings.join(" ").to_string()));
                }
                return Err(Error::Unknown);
            }
        }
    }

    async fn _multipart_request<T: DeserializeOwned>(
        &mut self,
        method: &str,
        url: &str,
        multipart: Form,
        query: Option<&[(&str, &str)]>,
    ) -> Result<T, Error> {
        let res = self
            .http
            .request(
                Method::from_str(method).unwrap_or(Method::GET),
                Url::parse_with_params(url, query.unwrap_or_default()).unwrap(),
            )
            .header(AUTHORIZATION, &self.auth.header(method, url, query))
            .multipart(multipart)
            .send()
            .await?
            .json::<Value>()
            .await?;

        if let Some(errors) = res.get("errors") {
            println!("got errors: {:?}", errors);
            return Err(Error::Unknown);
        }

        match serde_json::from_value::<T>(res) {
            Ok(data) => Ok(data),
            Err(_) => Err(Error::BadMedia),
        }
    }

    pub async fn me(&mut self, fields: Option<&[&str]>) -> Result<TwitterUserData, Error> {
        let fields_str = fields.map_or(String::new(), |f| f.join(","));
        let query = [("user.fields", fields_str.as_str())];

        self._request("GET", "https://api.twitter.com/2/users/me", Some(&query))
            .await
    }

    pub async fn upload_media(
        &mut self,
        path: &str,
        filename: Option<String>,
        mime: Option<String>
    ) -> Result<TwitterMediaResponse, Error> {
        let file_bytes;
        let detected_mime: Option<String>;
        if path.starts_with("http") {
            let media = reqwest::get(path).await?;
            let headers = media.headers().clone();
            let content_type_header = headers
                .get(reqwest::header::CONTENT_TYPE)
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            file_bytes = media.bytes().await?.to_vec();
            detected_mime = Some(content_type_header);
        } else {
            match fs::read(path) {
                Ok(bytes) => {
                    file_bytes = bytes;
                    detected_mime = infer::get(&file_bytes).map(|m| m.mime_type().to_string());
                }
                _ => return Err(Error::BadMedia),
            }
        }

        let mut chunked = false;
        let len = file_bytes.len();
        if len <= 1024 * 1024 {
            // simple upload
            let file_part = reqwest::multipart::Part::bytes(file_bytes)
                .file_name(filename.unwrap_or("media".into()));
            let form = reqwest::multipart::Form::new().part("media", file_part);

            self._multipart_request(
                "POST",
                "https://upload.twitter.com/1.1/media/upload.json",
                form,
                None,
            )
            .await
        } else {
            // chunked media upload
            chunked = true;
            let media_type = mime.unwrap_or_else(|| detected_mime.expect("Unable to infer media_type and no mime provided"));
            let init = self
                ._multipart_request::<TwitterMediaResponse>(
                    "POST",
                    "https://upload.twitter.com/1.1/media/upload.json",
                    reqwest::multipart::Form::new()
                        .text("command", "INIT")
                        .text("total_bytes", len.to_string())
                        .text("media_type", media_type.clone()),
                    None,
                )
                .await;

            let media_id = match init {
                Ok(data) => data.media_id_string,
                _ => return Err(Error::BadMedia),
            };

            for (i, chunk) in file_bytes.chunks(1024 * 1024).enumerate() {
                let append = self
                    .http
                    .post("https://upload.twitter.com/1.1/media/upload.json")
                    .header(
                        AUTHORIZATION,
                        &self.auth.header(
                            "POST",
                            "https://upload.twitter.com/1.1/media/upload.json",
                            None,
                        ),
                    )
                    .multipart(
                        reqwest::multipart::Form::new()
                            .text("command", "APPEND")
                            .text("media_id", media_id.to_string())
                            .text("segment_index", i.to_string())
                            .part(
                                "media",
                                reqwest::multipart::Part::bytes(chunk.to_vec())
                                    .file_name(format!("media_chunk_{}", i)),
                            ),
                    )
                    .send()
                    .await?
                    .status();

                if append != 204 {
                    return Err(Error::BadMedia);
                }
            }

            let mut finalize_form = reqwest::multipart::Form::new()
                .text("command", "FINALIZE")
                .text("media_id", media_id.to_string());

            if chunked {
                finalize_form = finalize_form.text("allow_async", "true");
            }

            let finalize = self
                ._multipart_request::<TwitterMediaResponse>(
                    "POST",
                    "https://upload.twitter.com/1.1/media/upload.json",
                    finalize_form,
                    None,
                )
                .await?;

            if finalize.processing_info.is_some() {
                loop {
                    let status = self
                        ._request_t::<TwitterMediaResponse>(
                            "GET",
                            "https://upload.twitter.com/1.1/media/upload.json",
                            Some(&[("command", "STATUS"), ("media_id", &media_id)]),
                        )
                        .await;

                    match status {
                        Ok(mut data) => match data.status() {
                            MediaStatus::InProgress => {
                                //println!("in progress");
                                tokio::time::sleep(tokio::time::Duration::from_secs(
                                    data.seconds_left(),
                                ))
                                .await;
                                continue;
                            }
                            MediaStatus::Succeeded => return Ok(data),
                            _ => return Err(Error::BadMedia),
                        },
                        _ => return Err(Error::BadMedia),
                    }
                }
            } else {
                Ok(finalize)
            }
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
            None,
        )
        .await
    }
}

#[derive(Debug, Deserialize)]
pub struct TwitterMediaResponseProcessingInfo {
    state: String,
    progress_percent: Option<u32>,
    check_after_secs: Option<u64>,
}

pub enum MediaStatus {
    InProgress,
    Succeeded,
    Failed,
    Bad,
}

#[derive(Debug, Deserialize)]
pub struct TwitterMediaResponse {
    media_id: u64,
    media_id_string: String,
    expires_after_secs: Option<u32>,
    processing_info: Option<TwitterMediaResponseProcessingInfo>,
}
impl TwitterMediaResponse {
    pub fn status(&mut self) -> MediaStatus {
        match &self.processing_info {
            Some(processing_info) => match processing_info.state.as_ref() {
                "in_progress" => MediaStatus::InProgress,
                "succeeded" => MediaStatus::Succeeded,
                "failed" => MediaStatus::Failed,
                _ => MediaStatus::Bad,
            },
            _ => MediaStatus::Bad,
        }
    }

    pub fn seconds_left(&mut self) -> u64 {
        self.processing_info
            .as_ref()
            .unwrap()
            .check_after_secs
            .unwrap_or(1)
    }

    pub fn id(&mut self) -> &str {
        &self.media_id_string
    }
}

#[derive(Debug, Deserialize)]
pub struct TwitterUserData {
    id: String,
    name: String,
    username: String,
    description: Option<String>,
    created_at: Option<String>,
}
impl TwitterUserData {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn has_description(&self) -> bool {
        self.description.is_some()
    }
    pub fn description(&self) -> &str {
        match &self.description {
            Some(description) => description,
            None => "",
        }
    }

    pub fn has_created_at(&self) -> bool {
        self.created_at.is_some()
    }
    pub fn created_at(&self) -> &str {
        match &self.created_at {
            Some(date) => date,
            None => "invalid",
        }
    }
}

#[derive(Debug, Deserialize)]
struct TwitterUserResponse {
    detail: Option<String>,
    data: Option<TwitterUserData>,
}
