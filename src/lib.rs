use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use futures::future::*;
use futures::sink::Sink;
use futures::sync::mpsc;
use log::info;
use reqwest;
use reqwest::r#async::Client;
use regex::Regex;
use serde::Deserialize;

pub mod error;

#[derive(Debug, Deserialize)]
pub struct SubArea {
    ports: Vec<String>,
    title: String,
}

#[derive(Debug, Clone)]
pub struct DocomoId((String, String, String));
#[derive(Debug, Clone)]
pub struct PortInfo(pub (String, String, usize));

pub struct HtmlSource {
    pub ports: Vec<PortInfo>,
    pub title: String,
}

impl HtmlSource {
    pub fn new(title: String) -> Self {
        HtmlSource {
            ports: vec!(),
            title: title,
        }
    }

    pub fn push(&mut self, port: PortInfo) {
        self.ports.push(port);
    }
}

fn parse_port_info(html: &str) -> HashMap<String, PortInfo> {
    let reg_str = r#"<form method="POST"(?s:.*?)name="ParkingID" value="(?s:(.*?))"(?s:.*?)<a class="port_list_btn_inner(?s:.*?)>(?s:(.*?))\.(?s:(.*?))<br>(?s:.*?)<br>([0-9]*?)台</a>"#;
    let reg = Regex::new(reg_str).unwrap();
    let caps = reg.captures_iter(&html);
    caps.filter_map(|cap| {
        let park_code = cap.get(1).map(|c| c.as_str().to_string());
        let park_id = cap.get(2).map(|c| c.as_str().to_string());
        let park_name = cap.get(3).map(|c| c.as_str().to_string());
        let cycles_num = cap.get(4).map(|c| c.as_str().parse::<usize>().unwrap());
        if let (Some(park_id), Some(park_name), Some(cycles_num)) = (park_id, park_name, cycles_num) {
            Some((park_id.to_string(), PortInfo((park_id, park_name, cycles_num))))
        } else {
            None
        }
    }).collect()
}

pub fn list_ports(docomo_id: &DocomoId, area_id: usize, filter_subareas: Vec<SubArea>, tx: Arc<Mutex<mpsc::Sender<HtmlSource>>>) -> impl Future<Item=(), Error=error::ErrorEnum> {
    let DocomoId((session_id, user_id, member_id)) = docomo_id;
    let area_id = area_id.to_string();
    let mut params = HashMap::new();
    params.insert("EventNo", "25702");
    params.insert("SessionID", &session_id);
    params.insert("UserID", &user_id);
    params.insert("MemberID", &member_id);
    //ページングせず全部取る。ポート数の上限を超えると2周目に入る。
    //2週目は何故か連番で来ず飛び飛びになる
    params.insert("GetInfoNum", "200");
    params.insert("GetInfoTopNum", "1");
    params.insert("MapType", "1");
    params.insert("MapZoom", "13");
    params.insert("EntServiceID", "TYO0001");
    params.insert("AreaID", &area_id);

    Client::new()
        .post("https://tcc.docomo-cycle.jp/cycle/TYO/cs_web_main.php")
        .form(&params)
        .send()
        .and_then(|mut e| {
            e.text()
        })
        .and_then(move |t| {
            let port_info_vec = parse_port_info(&t);
            for subarea in filter_subareas {
                let mut port_table_trs = HtmlSource::new(subarea.title);
                for key in subarea.ports {
                    let port = port_info_vec.get(&key);
                    if let Some(port) = port {
                        port_table_trs.push(port.clone());
                    }
                }
                let _ = (&mut *tx.lock().unwrap()).send(port_table_trs).wait();
            }
            Ok(())
        })
        .map_err(Into::into)
}

pub fn list_bicycles(docomo_id: DocomoId, parking_id: &str) -> impl Future<Item=(), Error=error::ErrorEnum> {
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
            let _bicycles: Vec<Option<String>> = caps.map(|cap| cap.get(1).map(|c| c.as_str().to_string())).collect();
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