use flate2::write::GzEncoder;
use flate2::Compression;
#[allow(unused_imports)]
use std::{
    collections::HashMap,
    env, error,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    result::Result::Err,
    result::Result::Ok,
};
use std::{
    error::Error,
    fs::{self, File},
    io::{BufRead, BufReader},
};

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("accepted new connection");
                std::thread::spawn(move || handle_connection(&mut stream));
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
    Ok(())
}

fn handle_connection(stream: &mut TcpStream) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut writer = stream.try_clone().unwrap();
    let mut reader = BufReader::new(stream);

    let mut buff = String::new();

    reader.read_line(&mut buff)?;
    let req_vec: Vec<String> = buff.split(" ").map(|s| s.to_string()).collect();
    let method = req_vec[0].clone();
    let path = req_vec[1].clone();

    let mut headers: HashMap<String, String> = HashMap::new();
    buff.clear();
    while reader.read_line(&mut buff)? > 0 {
        let line = buff.trim().to_string();

        if line.is_empty() {
            break;
        }

        if let Some((key, value)) = line.split_once(":") {
            headers.insert(key.to_string(), value.trim().to_string());
        }
        buff.clear();
    }

    let mut body = String::new();
    if let Some(length) = headers.get("Content-Length") {
        let size: usize = length.parse().unwrap_or(0);
        let mut buf = vec![0; size];

        reader.read_exact(&mut buf)?;
        body = String::from_utf8(buf)?;
    }

    let req = HttpRequst::new(method, path, headers, body);

    let rn = "\r\n";
    match req.method.as_str() {
        "GET" => {
            if req.path == "/" {
                writer.write_all(format!("HTTP/1.1 200 OK{}{}", rn, rn).as_bytes())?;
            } else if req.path.starts_with("/echo") {
                let str = req.path.trim_start_matches("/echo/");
                if req.headers.contains_key("Accept-Encoding") {
                    let encods_list: Vec<String> = req
                        .headers
                        .get("Accept-Encoding")
                        .unwrap()
                        .split(",")
                        .map(|s| s.trim().to_owned())
                        .collect();
                    if encods_list.contains(&"gzip".to_owned()) {
                        let mut e = GzEncoder::new(Vec::new(), Compression::default());
                        let data = str.as_bytes();
                        match e.write_all(data) {
                            Ok(_) => println!("wittien to e "),
                            Err(e) => println!("error wrting to e {}", e),
                        };
                        let d_data = match e.finish() {
                            Ok(data) => {
                                println!("e has finishd");
                                data
                            }
                            Err(e) => {
                                println!("Error finishing compression: {}", e);
                                return Err(e.into());
                            }
                        };

                        let response_header = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nContent-Encoding: gzip\r\n\r\n",
                            d_data.len()
                        );

                        match writer.write_all(response_header.as_bytes()) {
                            Ok(_) => println!("Response header sent"),
                            Err(e) => {
                                eprintln!("Error sending response header: {}", e);
                                return Err(e.into());
                            }
                        };

                        match writer.write_all(&d_data) {
                            Ok(_) => println!("Compressed data sent"),
                            Err(e) => {
                                eprintln!("Error sending compressed data: {}", e);
                                return Err(e.into());
                            }
                        }

                        match writer.flush() {
                            Ok(_) => println!("Response flushed successfully"),
                            Err(e) => {
                                eprintln!("Error flushing response: {}", e);
                                return Err(e.into());
                            }
                        }
                    }
                }
                writer
                    .write_all(
                        format!(
                            "HTTP/1.1 200 OK{}Content-Type: text/plain{}Content-Length: {}{}{}{}",
                            rn,
                            rn,
                            str.len(),
                            rn,
                            rn,
                            str
                        )
                        .as_bytes(),
                    )
                    .unwrap();
            } else if req.path.starts_with("/user-agent") {
                let agent = req
                    .headers
                    .get("User-Agent")
                    .inspect(|s| println!("got the user-agent: {}", s))
                    .expect("got an error");
                writer
                    .write_all(
                        format!(
                            "HTTP/1.1 200 OK{}Content-Type: text/plain{}Content-Length: {}{}{}{}",
                            rn,
                            rn,
                            agent.len(),
                            rn,
                            rn,
                            agent
                        )
                        .as_bytes(),
                    )
                    .unwrap();
            } else if req.path.starts_with("/files") {
                let file_path = req.path.trim_start_matches("/files/");
                let env_args: Vec<String> = env::args().collect();
                let mut dir = env_args[2].clone();
                dir.push_str(file_path);
                let file = fs::read(dir);
                match file {
                    Ok(fc) => {
                        writer.write_all(
                            format!(
                "HTTP/1.1 200 OK{}Content-Type: application/octet-stream{}Content-Length: {}{}{}",
                rn,
                rn,
                fc.len(),
                rn,
                rn,
            )
                            .as_bytes(),
                        )?;
                        writer.write_all(&fc)?;
                        return Ok(());
                    }
                    Err(e) => {
                        writer
                            .write_all(format!("HTTP/1.1 404 Not Found{}{}", rn, rn).as_bytes())?;
                        return Ok(());
                    }
                }
            } else {
                writer
                    .write_all(format!("HTTP/1.1 404 Not Found{}{}", rn, rn).as_bytes())
                    .unwrap();
            }
        }
        "POST" => {
            if req.path.starts_with("/files") {
                let file_path = req.path.trim_start_matches("/files/");
                let env_args: Vec<String> = env::args().collect();
                let mut dir = env_args[2].clone();
                dir.push_str(file_path);
                let file = File::create(dir);
                match file {
                    Ok(mut f) => {
                        f.write_all(req.body.as_bytes())?;
                        writer.write_all(format!("HTTP/1.1 201 Created{}{}", rn, rn).as_bytes())?;
                    }
                    Err(_e) => {
                        writer
                            .write_all(format!("HTTP/1.1 404 Not Found{}{}", rn, rn).as_bytes())?;
                    }
                }
            } else {
                writer
                    .write_all(format!("HTTP/1.1 404 Not Found{}{}", rn, rn).as_bytes())
                    .unwrap();
            }
        }
        _ => {
            unimplemented!()
        }
    }
    if req.path == "/" {
        writer.write_all(format!("HTTP/1.1 200 OK{}{}", rn, rn).as_bytes())?;
    } else if req.path.starts_with("/echo") {
        let str = req.path.trim_start_matches("/echo/");

        writer
            .write_all(
                format!(
                    "HTTP/1.1 200 OK{}Content-Type: text/plain{}Content-Length: {}{}{}{}",
                    rn,
                    rn,
                    str.len(),
                    rn,
                    rn,
                    str
                )
                .as_bytes(),
            )
            .unwrap();
    } else if req.path.starts_with("/user-agent") {
        let agent = req
            .headers
            .get("User-Agent")
            .inspect(|s| println!("got the user-agent: {}", s))
            .expect("got an error");
        writer
            .write_all(
                format!(
                    "HTTP/1.1 200 OK{}Content-Type: text/plain{}Content-Length: {}{}{}{}",
                    rn,
                    rn,
                    agent.len(),
                    rn,
                    rn,
                    agent
                )
                .as_bytes(),
            )
            .unwrap();
    } else if req.path.starts_with("/files") {
        let file_path = req.path.trim_start_matches("/files/");
        let env_args: Vec<String> = env::args().collect();
        let mut dir = env_args[2].clone();
        dir.push_str(file_path);
        let file = fs::read(dir);
        match file {
            Ok(fc) => {
                writer.write_all(
                    format!(
                "HTTP/1.1 200 OK{}Content-Type: application/octet-stream{}Content-Length: {}{}{}",
                rn,
                rn,
                fc.len(),
                rn,
                rn,
            )
                    .as_bytes(),
                )?;
                writer.write_all(&fc)?;
                return Ok(());
            }
            Err(e) => {
                writer.write_all(format!("HTTP/1.1 404 Not Found{}{}", rn, rn).as_bytes())?;
                return Ok(());
            }
        }
    } else {
        writer
            .write_all(format!("HTTP/1.1 404 Not Found{}{}", rn, rn).as_bytes())
            .unwrap();
    }
    Ok(())
}
#[derive(Debug)]
enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
}
#[derive(Debug)]
#[allow(dead_code)]
struct HttpRequst {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: String,
}

impl HttpRequst {
    fn new(method: String, path: String, headers: HashMap<String, String>, body: String) -> Self {
        HttpRequst {
            method,
            path,
            headers,
            body,
        }
    }
}
//impl TryFrom<Vec<String>> for HttpRequst {
//    type Error = HttpError;
//    fn try_from(request: Vec<String>) -> Result<Self, Self::Error> {
//        //println!("2");
//        println!("from TryFrom = {:?}", request);
//
//        let req_line = request[0].clone();
//
//        let req_lin_vec: Vec<&str> = req_line.split(" ").collect();
//        println!("req_lin_vec = {:?}", req_lin_vec);
//
//        //println!("get method = {:?}", req_lin_vec);
//
//        let headers = &request[1..request.len()].to_vec();
//        println!("headers = {:?}", headers);
//        let mut header_hash_map: HashMap<String, String> = HashMap::new();
//
//        for head in headers {
//            let header: Vec<String> = head.splitn(2, ":").map(|s| s.trim().to_string()).collect();
//
//            header_hash_map.insert(header[0].clone(), header[1].clone());
//            //println!("hasntable = {:?}", header_hash_map);
//            //println!("header_info = {:?}", header_info);
//            //println!("header = {}", header);
//        }
//
//        println!("headers = {:?}", headers);
//
//        Ok(HttpRequst {
//            method: match req_lin_vec[0] {
//                "GET" => HttpMethod::GET,
//                "POST" => HttpMethod::POST,
//                "PUT" => HttpMethod::PUT,
//                "DELETE" => HttpMethod::DELETE,
//                _ => {
//                    return Err(HttpError::InvalidFormat(
//                        "req method not supported".to_string(),
//                    ))
//                }
//            },
//            path: req_lin_vec[1].to_string(),
//            headers: header_hash_map,
//            body: "ppop".to_string(),
//        })
//    }
//}

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
enum HttpError {
    #[error("Invalid request format: {0}")]
    InvalidFormat(String),
    #[error("Can't handle status code: {0}")]
    UnsupportStatus(String),
    #[error("Can't handle content type: {0}")]
    UnsupportContentType(String),
    #[error("io exception.")]
    IoError(#[from] std::io::Error),
}
