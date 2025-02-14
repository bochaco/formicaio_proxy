use hyper::{body::Incoming, server::conn::http1, service::service_fn, Request, Response};
use hyper_util::rt::TokioIo;
use std::{error::Error, net::SocketAddr};
use tokio::net::{TcpListener, TcpStream};

const FORMICAIO_PROXY_PORT: &str = "FORMICAIO_PROXY_PORT";
const DEFAULT_FORMICAIO_PROXY_PORT: u16 = 52_100;

const FORMICAIO_ADDR: &str = "FORMICAIO_ADDR";
const DEFAULT_FORMICAIO_ADDR: &str = "127.0.0.1:3000";

async fn proxy_handler(
    req: Request<Incoming>,
    target_addr: String,
) -> Result<Response<Incoming>, Box<dyn Error + Send + Sync>> {
    let uri = format!("http://{target_addr}{}", req.uri().path());
    let url = uri.parse::<hyper::Uri>()?;
    //println!("Request forwarded to {url}");

    let stream = TcpStream::connect(target_addr).await?;
    let io = TokioIo::new(stream);

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            eprintln!("Connection failed: {:?}", err);
        }
    });

    let path = url.path();
    let builder = Request::builder().method(req.method()).uri(path);

    // Copy headers
    let headers = req.headers().clone();
    let mut req = builder.body(req.into_body())?;
    *req.headers_mut() = headers;

    let response = sender.send_request(req).await?;
    Ok(response)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let target_addr = std::env::var(FORMICAIO_ADDR).unwrap_or(DEFAULT_FORMICAIO_ADDR.to_string());
    println!("Requests to be forwarded to {target_addr}");

    let port = match std::env::var(FORMICAIO_PROXY_PORT) {
        Ok(port_str) => port_str.parse().unwrap_or(DEFAULT_FORMICAIO_PROXY_PORT),
        _ => DEFAULT_FORMICAIO_PROXY_PORT,
    };
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let listener = TcpListener::bind(addr).await?;
    println!("Listening on {addr} ...");

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        // Spawn a tokio task to serve multiple connections concurrently
        let target_addr = target_addr.clone();
        tokio::task::spawn(async move {
            // Finally, we bind the incoming connection to our service
            if let Err(err) = http1::Builder::new()
                // `service_fn` converts our function in a `Service`
                .serve_connection(
                    io,
                    service_fn(move |req| proxy_handler(req, target_addr.clone())),
                )
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}
