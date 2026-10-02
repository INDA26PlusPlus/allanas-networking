use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use axelwas_chess::{Color as ChessColor, PieceTypes, Place};

//   W\n" eller "B\n"
//   "A1B2P" + 64 board + "\n"
//   "OK\n"  "REJECT\n"  "CHECKMATE\n"  "STALEMATE\n"

pub struct Connection {
    pub stream: TcpStream,
    pub rx: Receiver<String>,
    pub disconnected: bool,
}

impl Connection {
    // https://doc.rust-lang.org/std/net/struct.TcpListener.html
    pub fn host() -> io::Result<(Connection, ChessColor)> {
        let listener = TcpListener::bind("0.0.0.0:6767")?;
        println!("waiting for opponent");

        let (mut stream, addr) = listener.accept()?;
        println!("opponent connected from {addr}");

        stream.write_all(b"B\n")?;
        let reader = BufReader::new(stream.try_clone()?);

        return Ok((Connection::start(stream, reader), ChessColor::White));
    }

    pub fn join(addr: &str) -> io::Result<(Connection, ChessColor)> {
        let addr = format!("{addr}:6767");
        let addr = addr.as_str();
        let stream = TcpStream::connect(addr)?;
        println!("connected to {addr}");

        // https://doc.rust-lang.org/std/io/struct.BufReader.html
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut line = String::new();
        reader.read_line(&mut line)?;

        let line = line.trim_end();
        let color;

        if line == "W" {
            color = ChessColor::White;
        } else if line == "B" {
            color = ChessColor::Black;
        } else {
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("not w or b")));
        }
        
        return Ok((Connection::start(stream, reader), color));
    }

    fn start(stream: TcpStream, mut reader: BufReader<TcpStream>) -> Connection {
        let _a = stream.set_nodelay(true);
        let (tx, rx) = mpsc::channel();

        // https://doc.rust-lang.org/std/thread/fn.spawn.html
        thread::spawn(move || loop {
            let mut line = String::new();
            let bytess = reader.read_line(&mut line);

            if bytess.is_ok() {
                if bytess.unwrap() == 0 {
                    let _ = tx.send("DISCONNECTED".to_string());
                    break;
                }
            } else {
                let _ = tx.send("DISCONNECTED".to_string());
                break;
            }

            let line = line.trim_end_matches(|c| c == '\x0a' || c == '\r').to_string();
            if tx.send(line).is_err() {
                break;
            }
        });

        return Connection {stream, rx, disconnected: false};
    }

    pub fn send(&mut self, msg: &str) {
        let result = self.stream.write_all(msg.as_bytes());

        if result.is_err() {
            println!("{:?}, {:?}", result.unwrap_err(), msg);
        }
    }

    pub fn polla(&mut self) -> Option<String> {
        if self.disconnected {
            return None;
        }
        let result = self.rx.try_recv();
        if result.is_ok() {
            let line = result.unwrap();

            if line == "DISCONNECTED" {
                self.disconnected = true;
            }

            return Some(line);
        } else {
            self.disconnected = true;

            return Some("DISCONNECTED".to_string());
        }
    }
}

pub fn move_into_protocol(from: Place, to: Place, promo: Option<PieceTypes>, board: &str) -> String {
    let mut boardS = String::with_capacity(70);
    let file = (b'A'+from.file as u8) as char;
    let rank = 8-from.row;
    boardS.push_str(&format!("{file}{rank}"));
    let file = (b'A'+to.file as u8) as char;
    let rank = 8-to.row;
    boardS.push_str(&format!("{file}{rank}"));

    let a;
    if promo.is_none() {
        a = '-';
    } else if promo.unwrap() == PieceTypes::Queen {
        a = 'Q';
    } else if promo.unwrap() == PieceTypes::Rook {
        a = 'R';
    } else if promo.unwrap() == PieceTypes::Bishop {
        a = 'B';
    } else {
        a = 'N';
    }
    boardS.push(a);
    boardS.push_str(board);
    boardS.push('\n');

    return boardS;
}

pub fn decoder3000(line: &str) -> Option<(Place, Place, Option<PieceTypes>, String)> {
    let c: Vec<char> = line.chars().collect();
    
    // A1B2P[char; 64]\n
    if c.len() != 69 {
        return None;
    }
    let file = c[0].to_ascii_uppercase();
    let file = file as usize - 'A' as usize;
    let rank = c[1] as usize - '0' as usize;
    let from = Place{row: 8-rank, file};

    let file = c[2].to_ascii_uppercase();
    let file = file as usize - 'A' as usize;
    let rank = c[3] as usize - '0' as usize;
    let to = Place{row: 8-rank, file};

    let a = c[4].to_ascii_uppercase();
    let aa;

    if a == '-' {
        aa = Some(None);
    } else if a == 'Q' {
        aa = Some(Some(PieceTypes::Queen));
    } else if a == 'R' {
        aa = Some(Some(PieceTypes::Rook));
    } else if a == 'B' {
        aa = Some(Some(PieceTypes::Bishop));
    } else if a == 'N' {
        aa = Some(Some(PieceTypes::Knight));
    } else {
        aa = None;
    }
    let promo = aa?;
    let board = c[5..].iter().collect();

    return Some((from, to, promo, board));
}

pub fn check_same_board(a: &str, b: &str) -> bool {
    let a: String = a.chars().map(|c| {
        if c.is_ascii_alphabetic() {
            c
        } else {
            '\x20'
        }
    }).collect();

    let b: String = b.chars().map(|c| {
        if c.is_ascii_alphabetic() {
            c
        } else {
            '\x20'
        }
    }).collect();

    if a == b {
        return true;
    } else {
        return false;
    }
}