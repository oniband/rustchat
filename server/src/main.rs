// STRUCTURE:
//          Iterate over connections
//              -> On connection
//                  -> Does User exist?
//                      N:
//                          -> Greet & initialize(add to vector of users)
//                      Y:
//                          -> Spawn a thread?
//                              -> thread waits for input
//                              -> send sanitized text out to all clients
//                              -> repeat?

// PACKET STRUCTURE: all packet types are 4 letters because that's easier :)
//                  --- FROM CLIENTS ---
//                  {NEWU username}
//                      -> RESPONSE: {USER uuid} OR {REJECTED}
//                  {MESS uuid message}
//                      -> RESPONSE: {GOOD} OR {REJECTED}
//                  {ALIV}
//                      -> RESPONSE: {ALIV} OR TIMEOUT
//                  {EXIT}
//                      -> RESPONSE: NONE
//                  --- FROM SERVER ---
//                  {USER uuid}
//                      -> responds with the users uuid
//                  {INFO message}
//                      -> Gives system information such as users joining or leaving
//                  {MESS message}
//                      -> regular messages from other users and themself
//                  {ALIV}
//                      -> a response from the server ping
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::str;
use std::sync::{Arc, Mutex};
use std::thread;
use uuid::Uuid;

const NEW_USER_PACKET_HEADER: &str = "NEWU";
const MESSAGE_PACKET_HEADER: &str = "MESS";
const DROP_CONNECTION_PACKET_HEADER: &str = "EXIT";

#[derive(Debug)]
struct UserInfo {
    user_id: Uuid,
    user_name: String,
    _user_address: SocketAddr,
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:420")?;
    let users: Arc<Mutex<Vec<UserInfo>>> = Arc::new(Mutex::new(Vec::new()));
    let streams: Arc<Mutex<Vec<TcpStream>>> = Arc::new(Mutex::new(Vec::new()));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let users_clone = Arc::clone(&users);
                let streams_clone = Arc::clone(&streams);
                let _ = thread::spawn(move || {
                    handle_client(stream, users_clone, streams_clone);
                });
            }
            Err(err) => eprintln!("Client has dropped, likely a timeout. err: {err}"),
        }
    }
    Ok(())
}

fn handle_client(
    mut stream: TcpStream,
    users: Arc<Mutex<Vec<UserInfo>>>,
    streams: Arc<Mutex<Vec<TcpStream>>>,
) {
    let mut response_buffer = [0; 128];
    let mut thread_user_name: String = String::new();
    let mut thread_user_id: String = String::new();
    let thread_ip_addr: SocketAddr = stream.peer_addr().unwrap();
    let mut user_initialized: bool = false;

    println!("Client Connected from {}", thread_ip_addr);

    {
        let mut lock = streams.lock().unwrap();
        lock.push(stream.try_clone().unwrap());
    }

    loop {
        match stream.read(&mut response_buffer) {
            Ok(0) => {
                if !user_initialized {
                    let mut streams_lock = streams.lock().unwrap();
                    streams_lock.retain(|stream| stream.peer_addr().unwrap() != thread_ip_addr);

                    stream.shutdown(std::net::Shutdown::Both).unwrap();
                    println!(
                        "Client from {} disconnected without creating a user",
                        thread_ip_addr
                    );
                    break;
                } else {
                    let users_clone = Arc::clone(&users);
                    remove_user_by_id(users_clone, String::from(&thread_user_id));

                    let streams_clone = Arc::clone(&streams);
                    remove_stream(streams_clone, thread_ip_addr);

                    stream.shutdown(std::net::Shutdown::Both).unwrap();
                    println!(
                        "User \"{}\" from {} disconnected by force-closing their client.",
                        thread_user_name, thread_ip_addr
                    );
                    break;
                }
            }
            Ok(_res) => {
                println!("");
            }
            Err(ref err)
                if err.kind() == io::ErrorKind::ConnectionReset
                    || err.kind() == io::ErrorKind::ConnectionAborted =>
            {
                stream.shutdown(std::net::Shutdown::Both).unwrap();
            }
            Err(err) => {
                eprintln!("{err}");
            }
        }

        let response_str_as_vec = match str::from_utf8(&response_buffer) {
            Ok(string) => string
                .trim_matches(char::from(0))
                .trim()
                .split(' ')
                .collect::<Vec<&str>>(),
            Err(err) => panic!("Invalid sequence! {}", err),
        };

        println!("{:?}", response_str_as_vec);

        match response_str_as_vec[0] {
            NEW_USER_PACKET_HEADER => {
                //Create a user and add it to the Arc
                let new_user_uuid: Uuid = Uuid::now_v7();
                let new_user: UserInfo = UserInfo {
                    user_id: new_user_uuid,
                    user_name: String::from(response_str_as_vec[1]),
                    _user_address: thread_ip_addr,
                };
                thread_user_name = String::from(response_str_as_vec[1]);
                thread_user_id = String::from(new_user_uuid);

                let mut global_user_vec = users.lock().unwrap();
                global_user_vec.push(new_user);
                user_initialized = true;
                println!(
                    "User from {} created with username: {} and uuid: {}",
                    thread_ip_addr, thread_user_name, thread_user_id
                );

                //Give their client their uuid
                write_to_stream_and_flush(&stream, &format!("USER {}\n", new_user_uuid)).unwrap();

                //Broadcast Entry to other clients
                let message = format!("{} Has entered the channel\n", thread_user_name);
                for stream in streams.lock().unwrap().iter() {
                    write_to_stream_and_flush(stream, &message).unwrap();
                }
            }
            MESSAGE_PACKET_HEADER => {
                //_validate_user(users, user_id, user_name)

                let message = format!(
                    "{} at {:?}: {}\n",
                    thread_user_name,
                    chrono::Local::now(),
                    response_str_as_vec[2..].join(" ")
                );

                for stream in streams.lock().unwrap().iter() {
                    write_to_stream_and_flush(stream, &message).unwrap();
                }
            }
            DROP_CONNECTION_PACKET_HEADER => {
                println!(
                    "User {:?} with ip of {:?} has decided to leave, farewell user :)",
                    thread_user_name, thread_ip_addr
                );

                let users_clone = Arc::clone(&users);
                remove_user_by_id(users_clone, String::from(&thread_user_id));

                let streams_clone = Arc::clone(&streams);
                remove_stream(streams_clone, thread_ip_addr);

                let message = format!("User {:?} has left the channel\n", thread_user_name);
                let streams_lock = streams.lock().unwrap();
                for stream in streams_lock.iter() {
                    write_to_stream_and_flush(stream, &message).unwrap();
                }

                stream.shutdown(std::net::Shutdown::Both).unwrap();
                break;
            }
            _ => {
                println!("Unknown Request \"{}\" ", response_str_as_vec[0]);
                write_to_stream_and_flush(&stream, "Unknown Request\n").unwrap();
            }
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

fn remove_user_by_id(users: Arc<Mutex<Vec<UserInfo>>>, user_id: String) {
    let mut user_lock = users.lock().unwrap();
    user_lock.retain(|x| x.user_id.to_string() != user_id);
}

fn remove_stream(streams: Arc<Mutex<Vec<TcpStream>>>, address: SocketAddr) {
    let mut streams_lock = streams.lock().unwrap();
    streams_lock.retain(|stream_closure| stream_closure.peer_addr().unwrap() != address);
}

fn _validate_user(_users: Arc<Mutex<Vec<UserInfo>>>, _user_id: String, _user_name: String) -> bool {
    todo!();
}
