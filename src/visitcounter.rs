use crate::database::Database;

use chrono::NaiveDateTime;
use std::future::{ready, Ready};
use std::rc::Rc;

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use futures_util::future::LocalBoxFuture;

#[derive(diesel::Insertable)]
#[diesel(table_name = crate::schema::visits)]
pub struct Visit {
    visitor: String,
    path: String,
    instance: NaiveDateTime,
}

pub struct VisitCounter {
    pub db: Database,
}

impl<S, B> Transform<S, ServiceRequest> for VisitCounter
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
    S: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = VisitCounterMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(VisitCounterMiddleware {
            db: self.db.clone(),
            service: Rc::new(service),
        }))
    }
}

pub struct VisitCounterMiddleware<S> {
    db: Database,
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for VisitCounterMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let head = req.head();
        let service = Rc::new(self.service.clone());
        let db_clone = self.db.clone();
        let connection_info = req.connection_info().clone();

        if let Some(address) = connection_info.realip_remote_addr() {
            let visit = Visit {
                visitor: address.to_string(),
                instance: chrono::Local::now().naive_local(),
                path: head.uri.to_string(),
            };

            Box::pin(async move {
                let _ = db_clone.new_visit(visit).await;
                service.call(req).await
            })
        } else {
            Box::pin(async move { service.call(req).await })
        }
    }
}
