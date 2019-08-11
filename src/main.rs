use std::env;
use std::collections::HashMap;

use futures::future::*;
use log::info;
use reqwest;
use reqwest::r#async::Client;
use regex::Regex;

mod error;

pub struct DocomoId((String, String, String));

pub fn list(docomo_id: DocomoId, parking_id: &str) -> impl Future<Item=(), Error=error::ErrorEnum> {
    let DocomoId((session_id, user_id, member_id)) = docomo_id;
    let mut params = HashMap::new();
    params.insert("EventNo", "25701");
    params.insert("SessionID", &session_id);
    params.insert("UserID", &user_id);
    params.insert("MemberID", &member_id);
    params.insert("GetInfoNum", "20");
    params.insert("GetInfoTopNum", "1");
    params.insert("ParkingEntID", "TYO");
    params.insert("ParkingID", parking_id);

    Client::new()
        .post("https://tcc.docomo-cycle.jp/cycle/TYO/cs_web_main.php")
        .form(&params)
        .send()
        .and_then(|mut e| {
            e.text()
        })
        .and_then(|t| {
            let reg_str = r#"<a(?s:.*?)>(.{7})</a>"#;
            let reg = Regex::new(reg_str).unwrap();
            let caps = reg.captures_iter(&t);
            let bicycles: Vec<Option<String>> = caps.map(|cap| cap.get(1).map(|c| c.as_str().to_string())).collect();
            info!("bicycles: {:?}", bicycles);
            Ok(())
        })
        .map_err(Into::into)
}

pub fn login(user_id: String, password: String) -> impl Future<Item=DocomoId, Error=error::ErrorEnum> {
    let mut params = HashMap::new();
    params.insert("EventNo", "21401");
    params.insert("SessionID", "ＰＯＳＴデータ");
    params.insert("MemberID", &user_id);
    params.insert("Password", &password);
    params.insert("MemAreaID", "3");

    Client::new()
        .post("https://tcc.docomo-cycle.jp/cycle/TYO/cs_web_main.php")
        .form(&params)
        .send()
        .and_then(|mut e| e.text())
        .and_then(|t| {
            let reg_str = r#"<input(?s:.*)name="SessionID" value="(?s:(.*?))">(?s:.*)<input(?s:.*)name="UserID" value="(?s:(.*?))">(?s:.*)<input(?s:.*)name="MemberID" value="(?s:(.*?))">"#;
            let reg = Regex::new(reg_str).unwrap();
            let caps = reg.captures(&t).unwrap();
            let session_id = caps.get(1).map(|m| m.as_str().to_string()).expect("SessionID not found");
            let user_id = caps.get(2).map(|m| m.as_str().to_string()).expect("UserID not found");
            let member_id = caps.get(3).map(|m| m.as_str().to_string()).expect("MemberID not found");
            Ok(DocomoId((session_id, user_id, member_id)))
        })
        .map_err(Into::into)
}

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
