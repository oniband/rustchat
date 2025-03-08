use std::io::{stdout, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::str;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

// STRUCTURE
// Three threads -> one that waits for input from the user and then sends it off to the server
//               -> one that reads packets from the server and then writes it to a vector of all stored inputs
//               -> one that handles the rendering of the text

const USER_RESPONSE_PACKET_HEADER: &str = "USER";
const MESSAGE_RESPONSE_PACKET_HEADER: &str = "MESS";

fn main() -> std::io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:25567")?;
    let application_should_close: AtomicBool = AtomicBool::new(false);

    let user_id = create_new_user(&mut stream, "oniband".to_string()).unwrap();
    println!("You have been assigned the uuid: {user_id} with the username oniband");

    while !application_should_close.load(Ordering::Relaxed) {
        _read_incoming_packets(&mut stream);
    }

    stream.shutdown(Shutdown::Both).unwrap();
    Ok(())
}

fn create_new_user(stream: &mut TcpStream, username: String) -> Result<String, ()> {
    stream.write(format!("NEWU {username}").as_bytes()).unwrap();

    let mut response_as_bytes = [0; 128];
    stream.read(&mut response_as_bytes).unwrap();
    let response_str_as_vec = match str::from_utf8(&response_as_bytes) {
        Ok(string) => string
            .trim_matches(char::from(0))
            .trim()
            .split(' ')
            .collect::<Vec<&str>>(),
        Err(err) => panic!("Invalid sequence! {}", err),
    };

    match response_str_as_vec[0] {
        USER_RESPONSE_PACKET_HEADER => return Ok(response_str_as_vec[1].to_string()),
        _ => return Err(()),
    }
}

fn _take_input() {}

fn _read_incoming_packets(stream: &mut TcpStream) {
    let mut response_as_bytes = [0; 128];
    stream.read(&mut response_as_bytes).unwrap();
    let response_str_as_vec = match str::from_utf8(&response_as_bytes) {
        Ok(string) => string
            .trim_matches(char::from(0))
            .trim()
            .split(' ')
            .collect::<Vec<&str>>(),
        Err(err) => panic!("Invalid sequence! {}", err),
    };
    match response_str_as_vec[0] {
        MESSAGE_RESPONSE_PACKET_HEADER => {
            println!("{}", response_str_as_vec[1..].join(" "));
        }
        _ => eprintln!("Not yet implemented protocol or garbage response from server :)"),
    }
}

fn _render() {}
