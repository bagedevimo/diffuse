#![feature(arbitrary_self_types, await_macro, async_await, proc_macro_hygiene)]

use futures::{compat::Executor01CompatExt, prelude::*};

fn main() {
    tarpc::init(tokio::executor::DefaultExecutor::current().compat());

    tokio::run(
        diffuse::proto::server::run(([0, 0, 0, 0], 11234).into())
            .map_err(|e| eprintln!("Oh no: {}", e))
            .boxed()
            .compat(),
    );
}
