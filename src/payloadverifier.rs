use hex::FromHex;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::future::{ready, Ready};
use std::rc::Rc;

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    error, web, Error,
};
use futures_util::future::LocalBoxFuture;

pub struct PayloadVerifier {
    pub mac: Hmac<Sha256>,
}

impl<S, B> Transform<S, ServiceRequest> for PayloadVerifier
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
    S: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = PayloadVerifierMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(PayloadVerifierMiddleware {
            mac: self.mac.clone(),
            service: Rc::new(service),
        }))
    }
}

pub struct PayloadVerifierMiddleware<S> {
    mac: Hmac<Sha256>,
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for PayloadVerifierMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let service = Rc::clone(&self.service);
        let mut mac = self.mac.clone();
        let body = req.extract::<web::Bytes>();

        Box::pin(async move {
            let body = body.await?;

            let Some(signature) = req.head().headers.get("X-Hub-Signature-256") else {
                return Err(error::ErrorBadRequest("No signature"));
            };

            mac.update(&body);

            // Sig string is sha256=deadbeef
            let real_signature = signature.to_str().unwrap()[7..].as_bytes();

            if mac
                .verify_slice(&Vec::from_hex(real_signature).unwrap())
                .is_err()
            {
                return Err(error::ErrorUnauthorized("Invalid signature"));
            }

            let (_, mut payload) = actix_http::h1::Payload::create(true);
            payload.unread_data(body);
            req.set_payload(payload.into());

            service.call(req).await
        })
    }
}
