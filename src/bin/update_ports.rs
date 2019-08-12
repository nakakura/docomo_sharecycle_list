use std::env;

use futures::future::*;

use docomo_sharecycle::*;

fn main() {
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    let user_id = env::var("USER_ID").expect("USER_ID is not set in environment variables");
    let password = env::var("PASSWORD").expect("PASSWORD is not set in environment variables");
    // FIXME とりあえず品川区役所前だけ調べている
    let fut = login(user_id, password)
        .and_then(|d_id| list(d_id, "10414"));
    tokio::run(fut.map_err(|e| panic!(e)));
}
