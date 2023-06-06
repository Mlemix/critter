mod oauth1;

pub use self::oauth1::*;

use rand::distributions::Alphanumeric;
use rand::Rng;

use std::time::{SystemTime, UNIX_EPOCH};
use base64::{engine::general_purpose, Engine as _};
use crypto::{hmac::Hmac, mac::Mac, sha1::Sha1};

#[derive(Debug)]
pub struct TwitterAuth {
    pin3: Option<Twitter3Pin>
}
impl TwitterAuth {
    pub fn from_oa1uc(
        consumer_key: &str,
        consumer_secret: &str,
        access_token: &str,
        access_token_secret: &str,
    ) -> Self {
        Self {
            pin3: Some(Twitter3Pin {
                consumer_key: consumer_key.into(),
                consumer_secret: consumer_secret.into(),
                access_token: access_token.into(),
                access_token_secret: access_token_secret.into() 
            })
        }
    }

    pub fn header(
        &mut self,
        method: &str,
        url: &str,
        //parameters: Vec<&str>,
        query: Option<&[(&str, &str)]>
    ) -> String {
        if let Some(pin3) = &self.pin3 {
            let nonce: String = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(42)
                .map(char::from)
                .collect();
            let time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("unix failed??")
                .as_secs()
                .to_string();

            let mut parameter_string = format!("oauth_consumer_key={}&oauth_nonce={}&oauth_signature_method=HMAC-SHA1&oauth_timestamp={}&oauth_token={}&oauth_version=1.0", 
                &pin3.consumer_key,
                &nonce, 
                &time,
                &pin3.access_token
            );
            if let Some(q) = query {
                let mut to_sort = q.iter()
                    .map(|(key, value)| format!("{}={}", urlencoding::encode(key), urlencoding::encode(value)))
                    .collect::<Vec<String>>();
                to_sort.push(parameter_string.clone());
                to_sort.sort();

                parameter_string = to_sort.join("&");
            }
            //}

            //println!("parameter string: {}", parameter_string);
            
            let sig_base_string = format!("{}&{}&{}",
                &method,
                urlencoding::encode(url),
                urlencoding::encode(&parameter_string)
            );

            //println!("sig_base_string: {}", sig_base_string);

            //println!("base string: {}", sig_base_string);

            let mut hmac = Hmac::new(Sha1::new(), format!("{}&{}", &pin3.consumer_secret, &pin3.access_token_secret).as_bytes());
            hmac.input(sig_base_string.as_bytes());
            let result = hmac.result();
            let code = result.code();

            format!("OAuth oauth_consumer_key=\"{}\",oauth_nonce=\"{}\",oauth_signature=\"{}\",oauth_signature_method=\"HMAC-SHA1\",oauth_timestamp=\"{}\",oauth_token=\"{}\",oauth_version=\"1.0\"", 
                &pin3.consumer_key,
                &nonce,
                urlencoding::encode(&general_purpose::STANDARD.encode(code)),
                &time,
                &pin3.access_token
            )
        } else {
            String::new()
        }
    }
}