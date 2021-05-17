#[macro_use]
extern crate dotenv_codegen;

use discord_game_sdk::{Discord, EventHandler, LobbyID, LobbyTransaction, User, UserID};
use serde_json::{json, Value};
use std::io::{
    stdin, stdout, BufRead, BufReader, BufWriter,
    ErrorKind::{ConnectionReset, WouldBlock},
    Write,
};
use std::net::{Shutdown, TcpStream};
use std::process;
use std::sync::{Arc, Mutex};
use std::{thread, time::Duration};

const MAX_VOLUME: f64 = 110.;

struct MyEventHandler {}

impl MyEventHandler {
    pub fn new() -> Self {
        Self {}
    }
}

impl EventHandler for MyEventHandler {
    fn on_member_connect(&mut self, discord: &Discord<Self>, _: LobbyID, member_id: UserID) {
        discord.set_local_volume(member_id, 0).unwrap();
    }
}

fn main() {
    let thread_link_mutex = Arc::new(Mutex::new(ThreadLink::new()));
    let mut discord =
        Discord::new(dotenv!("DISCORD_APPLICATION_ID").parse::<i64>().unwrap()).unwrap();
    let mut is_connection_start = false;
    let mut max_hearing_distance = 0.;

    *discord.event_handler_mut() = Some(MyEventHandler::new());

    loop {
        discord.run_callbacks().unwrap();

        if !is_connection_start {
            if let Ok(discord_user) = discord.current_user() {
                is_connection_start = true;

                thread::spawn(main_thread(thread_link_mutex.clone(), discord_user));
            }
        } else {
            let mut thread_link = thread_link_mutex.lock().unwrap();

            for (state, json_object) in thread_link.from.clone() {
                match state.as_str() {
                    "serverConfig" => {
                        max_hearing_distance = json_object["maxHearingDistance"].as_f64().unwrap();
                    }
                    "createLobby" => {
                        discord.create_lobby(&LobbyTransaction::new(), |discord, lobby_result| {
                            if let Ok(lobby) = lobby_result {
                                let id = lobby.id();
                                let mut thread_link = thread_link_mutex.lock().unwrap();

                                discord.connect_lobby_voice(id, |_, result| {
                                    if let Ok(_) = result {
                                        println!("Your now connected to the voice lobby");
                                    }
                                });

                                thread_link.to.push((
                                    "createLobby".to_string(),
                                    json!({
                                        "id": id,
                                        "secret": lobby.secret()
                                    }),
                                ));
                            }
                        });
                    }
                    "connectLobby" => {
                        let id = json_object["id"].as_i64().unwrap();
                        let secret = json_object["secret"].as_str().unwrap();

                        discord.connect_lobby(id, secret, move |discord, result| {
                            if let Ok(_) = result {
                                discord.connect_lobby_voice(id, move |discord, result| {
                                    if let Ok(_) = result {
                                        println!("Your now connected to the voice lobby");

                                        let mut iter_lobby_member_ids =
                                            discord.iter_lobby_member_ids(id).unwrap();

                                        while let Some(member_id) = iter_lobby_member_ids.next() {
                                            discord
                                                .set_local_volume(member_id.unwrap(), 0)
                                                .unwrap();
                                        }
                                    }
                                });
                            }
                        });
                    }
                    "sendPlayersPosition" => {
                        let mut personal_position = Position::default();
                        let mut other_position_vec = vec![];
                        let id = discord.current_user().unwrap().id();

                        for (id_string, position_json_object) in
                            json_object["positions"].as_object().unwrap()
                        {
                            let member_id = id_string.parse::<i64>().unwrap();

                            if id == member_id {
                                personal_position.set(position_json_object)
                            } else {
                                other_position_vec
                                    .push((member_id, Position::new(position_json_object)));
                            }
                        }

                        for (member_id, position) in other_position_vec {
                            let mut volume = 0.;

                            if personal_position.world == position.world {
                                let distance: f64 = ((personal_position.x - position.x).powi(2)
                                    + (personal_position.y - position.y).powi(2)
                                    + (personal_position.z - position.z).powi(2))
                                .sqrt();

                                volume = (1.
                                    - (distance.max(0.).min(max_hearing_distance)
                                        / max_hearing_distance))
                                    .powi(2)
                                    * MAX_VOLUME;
                            }

                            discord
                                .set_local_volume(member_id, volume.ceil() as u8)
                                .unwrap();
                        }
                    }
                    _ => {}
                }
            }

            thread_link.from = vec![];
        }

        thread::sleep(Duration::from_millis(1));
    }
}

struct ThreadLink {
    from: Vec<(String, Value)>,
    to: Vec<(String, Value)>,
}

impl ThreadLink {
    pub fn new() -> Self {
        Self {
            from: vec![],
            to: vec![],
        }
    }
}

#[derive(Debug)]
struct Position {
    world: String,
    x: f64,
    y: f64,
    z: f64,
}

impl Position {
    pub fn new(position_json_object: &Value) -> Self {
        let position = position_json_object.as_object().unwrap();

        Self {
            world: position["world"].as_str().unwrap().to_string(),
            x: position["x"].as_f64().unwrap(),
            y: position["y"].as_f64().unwrap(),
            z: position["z"].as_f64().unwrap(),
        }
    }

    pub fn default() -> Self {
        Self {
            world: "world".to_string(),
            x: 0.,
            y: 0.,
            z: 0.,
        }
    }

    pub fn set(&mut self, position_json_object: &Value) {
        let position = position_json_object.as_object().unwrap();

        self.world = position["world"].as_str().unwrap().to_string();
        self.x = position["x"].as_f64().unwrap();
        self.y = position["y"].as_f64().unwrap();
        self.z = position["z"].as_f64().unwrap();
    }
}

fn main_thread(thread_link_mutex: Arc<Mutex<ThreadLink>>, discord_user: User) -> impl Fn() {
    move || {
        print!("Enter the server address, or press enter to use the default address localhost : ");
        stdout().flush().unwrap();

        let mut address = get_input_from_stdin();

        if address == "" {
            address = "localhost".to_string()
        }

        print!("Enter the server port, or press enter to use the default port 25555 : ");
        stdout().flush().unwrap();

        let mut port = get_input_from_stdin();

        if port == "" {
            port = "25555".to_string()
        }

        match TcpStream::connect(format!("{}:{}", address, port)) {
            Ok(stream) => {
                let mut write_buffer = BufWriter::new(&stream);
                let mut read_buffer = BufReader::new(&stream);
                let mut text = String::new();

                loop {
                    match read_buffer.read_line(&mut text) {
                        Ok(_) => {
                            if text != "" {
                                let mut thread_link = thread_link_mutex.lock().unwrap();
                                let json_object: Value =
                                    serde_json::from_str(remove_end_newline(&text).as_str())
                                        .unwrap();
                                let state = json_object["state"].as_str().unwrap();

                                match state {
                                    "serverConfig" => {
                                        write_buffer.write_all(
                                            format!(
                                                "{{\"state\":\"discordUserInfo\",\"id\":{},\"username\":\"{}\",\"discriminator\":\"{}\"}}\r\n",
                                                discord_user.id(),
                                                discord_user.username(),
                                                discord_user.discriminator()
                                            ).as_bytes()
                                        ).unwrap();

                                        thread_link.from.push((state.to_string(), json_object))
                                    }
                                    "createLobby" | "connectLobby" => {
                                        thread_link.from.push((state.to_string(), json_object))
                                    }
                                    "linkCode" => {
                                        println!(
                                            "Command to link your Minecraft : /discordlinkmc link {}",
                                            json_object["code"].as_u64().unwrap()
                                        );

                                        thread::spawn(exit_thread(stream.try_clone().unwrap()));
                                    }
                                    "sendPlayersPosition" => {
                                        thread_link.from.push((state.to_string(), json_object))
                                    }
                                    "end" => exit(&stream),
                                    _ => {}
                                }

                                for (state, json_object) in thread_link.to.clone() {
                                    match state.as_str() {
                                        "createLobby" => write_buffer.write_all(
                                            format!(
                                                "{{\"state\":\"createLobby\",\"id\":{},\"secret\":\"{}\"}}\r\n",
                                                json_object["id"].as_u64().unwrap(),
                                                json_object["secret"].as_str().unwrap()
                                            ).as_bytes()
                                        ).unwrap(),
                                        _ => {}
                                    }
                                }

                                thread_link.to = vec![];
                                write_buffer.flush().unwrap()
                            }
                        }
                        Err(ref e) if e.kind() == WouldBlock => println!("Would block"),
                        Err(ref e) if e.kind() == ConnectionReset => process::exit(0),
                        Err(e) => println!("Encountered IO error: {}", e),
                    }

                    text = String::new();

                    thread::sleep(Duration::from_millis(50));
                }
            }
            Err(e) => {
                println!("Failed to connect: {}", e);

                process::exit(0);
            }
        }
    }
}

fn exit_thread(stream: TcpStream) -> impl Fn() {
    move || {
        println!("Press any key to end the program...");

        while let Err(_) = stdin().read_line(&mut String::new()) {}

        exit(&stream);
    }
}

fn exit(stream: &TcpStream) {
    let mut write_buffer = BufWriter::new(stream);

    write_buffer.write_all(b"{\"state\":\"end\"}\r\n").unwrap();
    write_buffer.flush().unwrap();

    stream.shutdown(Shutdown::Both).unwrap();

    process::exit(0);
}

fn remove_end_newline(value: &String) -> String {
    value
        .trim_end_matches('\n')
        .trim_end_matches('\r')
        .to_string()
}

fn get_input_from_stdin() -> String {
    let mut value = String::new();

    while let Err(_) = stdin().read_line(&mut value) {}

    remove_end_newline(&value)
}
