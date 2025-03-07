use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::str;
use std::thread;

// STRUCTURE
// Three threads -> one that waits for input from the user and then sends it off to the server
//               -> one that reads packets from the server and then writes it to a vector of all stored inputs
//               -> one that handles the rendering of the text

fn _take_input() {}

fn _read_incoming_packets() {}

fn _render() {}

fn main() -> std::io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:25567")?;

    stream.write("NEWU oniband".as_bytes())?;

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

    println!("Recieved from server: {:?}", response_str_as_vec.join(" "));
    stream.shutdown(Shutdown::Both).unwrap();
    Ok(())
}
