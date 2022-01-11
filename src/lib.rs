use std::{
    collections::HashMap,
    fmt::Debug,
    future::{ready, Future},
    pin::Pin,
    sync::{Arc, Mutex},
};

use http_client::{http_types::StatusCode, Error, HttpClient, Request, Response};

#[derive(Debug, Default)]
pub struct MockHttpServer(Arc<Mutex<State>>);

impl MockHttpServer {
    pub fn new() -> MockHttpServer {
        MockHttpServer::default()
    }

    pub fn client(&self) -> MockHttpClient {
        MockHttpClient(self.0.clone())
    }

    pub fn handle_default(&self, default_handler: impl HandlerFn) {
        self.0.lock().unwrap().default_handler = Some(shared_handler_fn(default_handler));
    }

    pub fn handle_path(&self, path: impl Into<String>, handler: impl HandlerFn) {
        self.0
            .lock()
            .unwrap()
            .path_handlers
            .insert(path.into(), shared_handler_fn(handler));
    }
}

#[derive(Debug)]
pub struct MockHttpClient(Arc<Mutex<State>>);

impl HttpClient for MockHttpClient {
    fn send<'life0, 'async_trait>(
        &self,
        req: Request,
    ) -> Pin<Box<dyn Future<Output = Result<Response, Error>> + Send>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let state = self.0.lock().unwrap();
        Box::pin(ready(state.respond(req)))
    }
}

#[derive(Default)]
struct State {
    default_handler: Option<SharedHandlerFn>,
    path_handlers: HashMap<String, SharedHandlerFn>,
}

impl State {
    fn respond(&self, req: Request) -> Result<Response, Error> {
        match self
            .path_handlers
            .get(req.url().path())
            .or(self.default_handler.as_ref())
        {
            Some(handler) => handler.lock().unwrap()(req),
            None => Ok(Response::new(StatusCode::NotFound)),
        }
    }
}

impl Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field(
                "default_handler",
                &self.default_handler.as_ref().and(Some(())),
            )
            .field("path_handlers", &self.path_handlers.keys())
            .finish()
    }
}

pub trait HandlerFn: FnMut(Request) -> Result<Response, Error> + Send + 'static {}

impl<F: FnMut(Request) -> Result<Response, Error> + Send + 'static> HandlerFn for F {}

type SharedHandlerFn = Mutex<Box<dyn HandlerFn>>;

fn shared_handler_fn(f: impl HandlerFn) -> SharedHandlerFn {
    Mutex::new(Box::new(f))
}
