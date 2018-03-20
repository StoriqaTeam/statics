// rusoto_core::request::HttpClient
// https://rusoto.github.io/rusoto/rusoto_core/request/struct.HttpClient.html

// https://rusoto.github.io/rusoto/rusoto_core/trait.ProvideAwsCredentials.html

// impl<P, D> S3Client<P, D>
// where
//     P: ProvideAwsCredentials,
//     D: DispatchSignedRequest,
// {
//     pub fn new(request_dispatcher: D, credentials_provider: P, region: region::Region) -> Self {
//         S3Client {
//             inner: ClientInner::new(credentials_provider, request_dispatcher),
//             region: region,
//         }
//     }
// }


pub mod credentials;
