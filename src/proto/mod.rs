// pub use client::Client;
// pub use server::Server;

tarpc::service! {
    rpc hello(name: String) -> String;
    rpc store_blob(blob: crate::git::Record) -> bool;
}

pub mod server;
