use futures::IntoFuture;

use actix_http::{h1, http::Method};
use actix_server::Server;
use actix_web2::{middleware, App, Error, HttpRequest, Resource};

fn index(req: HttpRequest) -> &'static str {
    println!("REQ: {:?}", req);
    "Hello world!\r\n"
}

fn index_async(req: HttpRequest) -> impl IntoFuture<Item = &'static str, Error = Error> {
    println!("REQ: {:?}", req);
    Ok("Hello world!\r\n")
}

fn no_params() -> &'static str {
    "Hello world!\r\n"
}

fn main() {
    ::std::env::set_var("RUST_LOG", "actix_server=info,actix_web2=info");
    env_logger::init();
    let sys = actix_rt::System::new("hello-world");

    Server::build()
        .bind("test", "127.0.0.1:8080", || {
            h1::H1Service::new(
                App::new()
                    .middleware(
                        middleware::DefaultHeaders::new().header("X-Version", "0.2"),
                    )
                    .service(
                        Resource::build("/resource1/index.html")
                            .method(Method::GET)
                            .to(index),
                    )
                    .service(
                        Resource::build("/resource2/index.html").to_async(index_async),
                    )
                    .service(Resource::build("/test1.html").to(|| "Test\r\n"))
                    .service(Resource::build("/").to(no_params)),
            )
        })
        .unwrap()
        .workers(1)
        .start();

    let _ = sys.run();
}
