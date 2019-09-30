use std::env;
use std::fs;
use std::io::{Write, Read, BufWriter, BufReader, copy};
use std::sync::{Arc, Mutex};

use futures::future::*;
use futures::stream::Stream;
use log::info;
use toml;
use serde::Deserialize;

use docomo_sharecycle::*;

#[derive(Debug, Deserialize)]
struct Area {
    id: usize,
    subarea: Vec<SubArea>,
}

#[derive(Debug, Deserialize)]
struct Toml {
    area: Vec<Area>,
}

fn read_file(path: String) -> Result<String, String> {
    let mut file_content = String::new();

    let mut fr = fs::File::open(path)
        .map(|f| BufReader::new(f))
        .map_err(|e| e.to_string())?;

    fr.read_to_string(&mut file_content)
        .map_err(|e| e.to_string())?;

    Ok(file_content)
}

fn main() {
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    let (tx, rx) = futures::sync::mpsc::channel::<HtmlSource>(100);
    let tx_arc = Arc::new(Mutex::new(tx));
    let user_id = env::var("USER_ID").expect("USER_ID is not set in environment variables");
    let password = env::var("PASSWORD").expect("PASSWORD is not set in environment variables");
    let toml_path = env::var("TOML_PATH").expect("PORTS_PATH is not set in environment variables");
    let file_dir = env::var("FILE_DIR").expect("PORTS_PATH is not set in environment variables");
    let areas = {
        let s = match read_file("./config.toml".to_owned()) {
            Ok(s) => s,
            Err(e) => panic!("fail to read file: {}", e),
        };
        let config: Result<Toml, toml::de::Error> = toml::from_str(&s);
        config.expect("toml parse error").area
    };

    let fut = login(user_id, password)
        .and_then(move |d_id| {
            for area in areas {
                let area_id = area.id;
                let list = list_ports(&d_id, area_id, area.subarea, tx_arc.clone());
                tokio::spawn(list.map_err(|e| panic!(e)));
            }
            Ok(())
        }); //area_id: 10 は品川区, 3は港区
    tokio::run(fut.map_err(|e| panic!(e)));

    let html =
r#"<!DOCTYPE html><html lang="en">
<head>
<meta http-equiv="Content-Type" content="text/html; charset=utf-8"/>
<table border=1>"#.to_string();
    let rx = rx.and_then(move |html_source| {
        let mut output= html.clone();
        let title = html_source.title;
        for port in html_source.ports {
            let PortInfo((port_id, port_name, cycles_num)) = port;
            let message = format!("\n<tr><td>{}.{}</td><td>{}</td></tr>", port_id, port_name, cycles_num);
            output += &message;
        }
        output += &format!("\n</table>\n<img src='./{}.png'>\n</body>\n</html>", title);
        Ok((title, output))
    }).for_each(move |(title, html)| {
        let file_path = format!("{}/{}.html", file_dir, title);
        let mut f = fs::File::create(&file_path).unwrap(); // open file, you can write to file.
        f.write_all(html.as_bytes()).unwrap(); // byte-only
        Ok(())
    });
    tokio::run(rx);
}
