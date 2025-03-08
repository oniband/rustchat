use std::io::{self, BufRead, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::str;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

// STRUCTURE
// Three threads -> one that waits for input from the user and then sends it off to the server
//               -> one that reads packets from the server and then writes it to a vector of all stored inputs
//               -> one that handles the rendering of the text

const USER_RESPONSE_PACKET_HEADER: &str = "USER";
const MESSAGE_RESPONSE_PACKET_HEADER: &str = "MESS";
const USER_EXIT_OKAY_RESPONSE_PACKET_HEADER: &str = "EXIT";

fn main() -> std::io::Result<()> {
    let mut stream: TcpStream = TcpStream::connect("127.0.0.1:25567")?;
    let program_should_exit: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

    let user_id = create_new_user(&mut stream, "oniband".to_string()).unwrap();
    //println!("You have been assigned the uuid: {user_id} with the username oniband");

    let stream_read_thread_clone = stream.try_clone().unwrap();
    let program_should_exit_read_clone = Arc::clone(&program_should_exit);
    let read_thread = thread::spawn(move || {
        read_incoming_packets(stream_read_thread_clone, program_should_exit_read_clone);
    });

    let stream_user_input_thread_clone = stream.try_clone().unwrap();
    let program_should_exit_user_input_clone = Arc::clone(&program_should_exit);
    let write_thread = thread::spawn(move || {
        read_user_input_and_send(
            stream_user_input_thread_clone,
            &user_id,
            program_should_exit_user_input_clone,
        );
    });

    read_thread.join().unwrap();
    write_thread.join().unwrap();

    //stream.shutdown(Shutdown::Both).unwrap();
    Ok(())
}

fn create_new_user(stream: &mut TcpStream, username: String) -> Result<String, ()> {
    let message = format!("NEWU {username}");
    write_to_stream_and_flush(&stream, &message).unwrap();
    stream.flush().unwrap();

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

fn read_user_input_and_send(
    mut stream: TcpStream,
    user_id: &str,
    program_should_exit: Arc<AtomicBool>,
) {
    while !program_should_exit.load(Ordering::Relaxed) {
        let mut input = String::new();
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut input).unwrap();
        io::stdout().flush().unwrap();

        let split_command = input.trim().split("/").collect::<Vec<&str>>();

        println!("{split_command:?}");
        if split_command.len() == 2 {
            match split_command[1] {
                "exit" | "quit" => {
                    println!("User exits here");
                    program_should_exit.store(true, Ordering::Relaxed);
                    let message = format!("EXIT {user_id}");
                    write_to_stream_and_flush(&stream, &message).unwrap();
                    continue;
                }
                _ => {
                    println!("Unregistered command");
                    continue;
                }
            }
        }

        let split_input = input.trim().split(" ").collect::<Vec<&str>>();

        println!("TEST: {:?}", split_input.join(" "));

        match split_input[0] {
            "" => continue,
            " " => continue,
            _ => {
                let message = format!("MESS {}", split_input.join(""));
                write_to_stream_and_flush(&stream, &message).unwrap();
            }
        };
    }
}

fn read_incoming_packets(mut stream: TcpStream, program_should_exit: Arc<AtomicBool>) {
    while !program_should_exit.load(Ordering::Relaxed) {
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
            USER_EXIT_OKAY_RESPONSE_PACKET_HEADER => {
                println!("Gracefully exited from server :)");
                continue;
            }
            "" => {
                println!("Server is kill...");
                break;
            }
            _ => eprintln!(
                "Not yet implemented protocol or garbage response from server :) {}",
                response_str_as_vec[0]
            ),
        }
    }
}

fn write_to_stream_and_flush(mut stream: &TcpStream, message: &str) -> Result<(), ()> {
    let _ = stream.write(message.as_bytes()).map_err(|err| {
        eprintln!("Failed to write to stream with error {err}");
    });
    let _ = stream.flush().map_err(|err| {
        eprintln!(
            "Failed to flush stream {:?} with error {}",
            stream.peer_addr(),
            err
        );
    });
    Ok(())
}
