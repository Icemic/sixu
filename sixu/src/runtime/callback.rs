use std::future::Future;
use std::pin::Pin;

use crate::format::CommandLine;

pub type OnCommandHandler = Box<
    dyn (FnMut(&CommandLine) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>) + Send + Sync,
>;
