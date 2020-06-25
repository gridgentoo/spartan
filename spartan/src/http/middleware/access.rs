use std::pin::Pin;
use std::task::{Context, Poll};

use crate::config::Config;
use actix_service::{Service, Transform};
use actix_web::{
    dev::ServiceRequest, dev::ServiceResponse, http::StatusCode, Error, ResponseError,
};
use futures_util::future::{ok, Ready};
use std::future::Future;
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum AccessError {
    #[error("Access denied")]
    AccessDenied,
    #[error("Authorization header not found")]
    AuthorizationHeaderNotFound,
    #[error("Incorrect key header")]
    IncorrectKeyHeader,
}

impl ResponseError for AccessError {
    fn status_code(&self) -> StatusCode {
        match self {
            AccessError::AccessDenied => StatusCode::UNAUTHORIZED,
            AccessError::AuthorizationHeaderNotFound | AccessError::IncorrectKeyHeader => {
                StatusCode::BAD_REQUEST
            }
        }
    }
}

pub struct Access {
    config: &'static Config,
}

impl Access {
    pub fn new(config: &'static Config) -> Access {
        Access { config }
    }
}

impl<S, B> Transform<S> for Access
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AccessMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AccessMiddleware {
            service,
            config: self.config,
        })
    }
}

pub struct AccessMiddleware<S> {
    service: S,
    config: &'static Config,
}

#[allow(clippy::type_complexity)]
impl<S, B> Service for AccessMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        if let Err(error) = self.parse_request(&req) {
            Box::pin(async move { Err(error.into()) })
        } else {
            let fut = self.service.call(req);

            Box::pin(async move { Ok(fut.await?) })
        }
    }
}

impl<S> AccessMiddleware<S> {
    fn parse_request(&self, req: &ServiceRequest) -> Result<(), AccessError> {
        if self.has_access_keys() {
            if let Some(queue) = req.match_info().get("queue") {
                //TODO: Authorization header parsing

                let key = req
                    .headers()
                    .get("Authorization")
                    .ok_or_else(|| AccessError::AuthorizationHeaderNotFound)?
                    .to_str()
                    .map_err(|_| AccessError::IncorrectKeyHeader)?;

                if self.check_access(key, queue) {
                    Ok(())
                } else {
                    Err(AccessError::AccessDenied)
                }
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    fn check_access(&self, key: &str, queue: &str) -> bool {
        let keys = self
            .config
            .access_keys
            .as_ref()
            .expect("Config doesn't have access keys");

        if let Some(key) = keys.get(key) {
            key.has_queue(queue)
        } else {
            false
        }
    }

    fn has_access_keys(&self) -> bool {
        self.config.access_keys.is_some()
    }
}
