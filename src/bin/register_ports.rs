use std::env;
use std::fs;
use std::io::{Write, Read, BufWriter, BufReader, copy};
use std::sync::{Arc, Mutex};

use futures::future::*;
use futures::stream::Stream;
use log::info;

use docomo_sharecycle::*;

fn main() {
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    let (tx, rx) = futures::sync::mpsc::channel::<PortInfo>(100);
    let tx_arc = Arc::new(Mutex::new(tx));
    let user_id = env::var("USER_ID").expect("USER_ID is not set in environment variables");
    let password = env::var("PASSWORD").expect("PASSWORD is not set in environment variables");
    let file_path = env::var("FILE_PATH").expect("PASSWORD is not set in environment variables");
    // FIXME とりあえず品川区役所とシーバンス周辺だけ調べる
    // "10414", "I1-01.品川区役所 本庁舎前"
    // "10415", "I1-02.品川区役所 第三庁舎前"
    // "10416", "I1-03.大井町駅中央口（西側）"
    // "10417", "I1-04.大井町歩道橋"
    // "10580", "I1-30.ローソン 大井三丁目店"
    // "10668", "I1-44.大井三ツ又商店街入口"
    // "10772", "I1-58.品川区役所\u{3000}第二庁舎前"
    // "10461", "I1-09.東品川海上公園第1"
    // "10516", "I1-17.東品川海上公園第2"
    // "10305", "C5-17.鈴与浜松町ビル"
    // "10576", "C5-27.プレミア海岸ビル"
    // "10166", "C5-10.シーバンス"
    // "10091", "C5-05.浜松町ビルディング"

    let minato_ports: Vec<String> = vec!("10305", "10576", "10166", "10091").into_iter().map(|s| s.to_string()).collect();
    let shinagwa_ports: Vec<String> = vec!("10414", "10415", "10416", "10417", "10580", "10668", "10772", "10461", "10516").into_iter().map(|s| s.to_string()).collect();
    let fut = login(user_id, password)
        .and_then(move |d_id| {
            let list1 = list_ports(&d_id, "3", minato_ports, tx_arc.clone());
            tokio::spawn(list1.map_err(|e| panic!(e)));
            let list2 = list_ports(&d_id, "10", shinagwa_ports, tx_arc.clone());
            tokio::spawn(list2.map_err(|e| panic!(e)));
            Ok(())
        }); //area_id: 10 は品川区, 3は港区
    tokio::run(fut.map_err(|e| panic!(e)));

    let rx = rx.fold("<table border=1>".to_string(), |sum, port| {
        let PortInfo((_, port_name, cycles_num)) = port;
        let message = format!("{}<tr><td>{}</td><td>{}</td></tr>", sum, port_name, cycles_num);
        Ok(message)
    }).and_then(move |message| {
        let message = format!("{}</table>", message);
        let mut f = fs::File::create(&file_path).unwrap(); // open file, you can write to file.
        f.write_all(message.as_bytes()).unwrap(); // byte-only
        Ok(())
    });
    tokio::run(rx);
}
