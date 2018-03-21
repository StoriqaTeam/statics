// // https://rusoto.github.io/rusoto/rusoto_core/trait.ProvideAwsCredentials.html

use rusoto_core::{AwsCredentials, CredentialsError, ProvideAwsCredentials};
use futures::future::{ok, FutureResult};

pub struct Credentials {
    key: String,
    secret: String,
}

impl Credentials {
    pub fn new(key: String, secret: String) -> Self {
        Self { key, secret }
    }
}

impl ProvideAwsCredentials for Credentials {
    type Future = FutureResult<AwsCredentials, CredentialsError>;

    fn credentials(&self) -> Self::Future {
        ok(AwsCredentials::new(
            self.key.clone(),
            self.secret.clone(),
            None,
            None,
        ))
    }
}
