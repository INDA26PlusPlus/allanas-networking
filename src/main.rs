use axelwas_chess::{Color as ChessColor, EndStates, PieceTypes, Place};
use crate::wrapper::{Game, PROMOTIONS_CHOICESAA};
use crate::network::Connection;
mod network;
mod wrapper;

use std::collections::HashMap;

use ggez::glam::Vec2;
use ggez::graphics::{self, Color, DrawParam, Quad, Rect, Text, TextFragment,PxScale};
use ggez::input::mouse::MouseButton;
use ggez::{event, Context, GameError, GameResult};

pub const SQUARE_SIZE: f32 = 80.0;
pub const BOARD: f32 = SQUARE_SIZE * 8.0;

const LIGHT: Color = Color {
    r: 0.94,
    g: 0.85,
    b: 0.71,
    a: 1.0
};
const DARK: Color = Color {
    r: 0.71,
    g: 0.53,
    b: 0.39,
    a: 1.0
};
const PANEL: Color = Color {
    r: 0.14,
    g: 0.14,
    b: 0.18,
    a: 1.0
};

pub struct ChessGui {
    game: Game,
    sprites: HashMap<char, graphics::Image>,
    selected_piece: Option<Place>,
    legalmoves: Vec<Place>,
    pending: Option<(Place, Place)>,
    text: Option<Text>,

    netty: Connection,
    my_color: ChessColor,
    waiting: Option<(Place, Place, Option<PieceTypes>)>,
    game_over: bool,
}

impl ChessGui {
    pub fn new(ctx: &mut Context, netty: Connection, my_color: ChessColor) -> GameResult<ChessGui> {
        let mut sprites = HashMap::new();
        sprites.insert('P', graphics::Image::from_path(ctx, "/wp.png")?);
        sprites.insert('R', graphics::Image::from_path(ctx, "/wr.png")?);
        sprites.insert('N', graphics::Image::from_path(ctx, "/wn.png")?);
        sprites.insert('B', graphics::Image::from_path(ctx, "/wb.png")?);
        sprites.insert('Q', graphics::Image::from_path(ctx, "/wq.png")?);
        sprites.insert('K', graphics::Image::from_path(ctx, "/wk.png")?);

        sprites.insert('p', graphics::Image::from_path(ctx, "/bp.png")?);
        sprites.insert('r', graphics::Image::from_path(ctx, "/br.png")?);
        sprites.insert('n', graphics::Image::from_path(ctx, "/bn.png")?);
        sprites.insert('b', graphics::Image::from_path(ctx, "/bb.png")?);
        sprites.insert('q', graphics::Image::from_path(ctx, "/bq.png")?);
        sprites.insert('k', graphics::Image::from_path(ctx, "/bk.png")?);

        println!("{:?}", sprites);

        return Ok(ChessGui {
            game: Game::new(),
            sprites,
            selected_piece: None,
            legalmoves: Vec::new(),
            pending: None,
            text: None,

            netty,
            my_color,
            waiting: None,
            game_over: false,
        })
    }

    fn select(&mut self, place: Place) {
        let piece = self.game.piece_at(place);

        if piece.is_some() {
            let piece = piece.unwrap();

            if piece.color == self.game.turn() {
                self.selected_piece = Some(place);
                self.legalmoves = self.game.legal_moves(place);
                return;
            }
        }

        self.selected_piece = None;
        self.legalmoves.clear();
        
        return;
    }

    fn place(&self, x: f32, y: f32) -> Option<Place> {
        if x < 0.0 || y < 0.0 || x >= BOARD || y >= BOARD {
            return None;
        }
        return Some(Place {row: (y / SQUARE_SIZE) as usize, file: (x / SQUARE_SIZE) as usize})
    }

    fn actuallyplay(&mut self, from: Place, to: Place, promote_to: Option<PieceTypes>) {
        self.game.play(from, to, promote_to);
        self.pending = None;
        self.selected_piece = None;

        return;
    }

    // helper
    fn show_text(&mut self, msg: &str) {
        // https://github.com/ggez/ggez/blob/9c865474504084b60204874bc028ba91c5f013b9/examples/text.rs#L45
        let text = Text::new(TextFragment {
            text: msg.to_string(),
            color: Some(Color::new(0.0, 0.0, 1.0, 1.0)),
            font: Some("LiberationMono-Regular".into()),
            scale: Some(PxScale::from(50.0)),
            ..Default::default()
        });
        self.text = Some(text);
    }

    fn send_move(&mut self, from: Place, to: Place, promote_to: Option<PieceTypes>) {
        self.pending = None;
        self.selected_piece = None;
        self.legalmoves.clear();

        if self.game.board_previewing(from, to, promote_to).is_none() {
            return;
        }

        let board = self.game.board_previewing(from, to, promote_to).unwrap();
        let a = network::move_into_protocol(from, to, promote_to, &board);
        self.netty.send(&a);
        self.waiting = Some((from, to, promote_to));
    }

    fn check_end(&mut self) {
        let status = self.game.status();
        if status == EndStates::Checkmate {
            if self.game.turn() == ChessColor::White {
                println!("Checkmate! Black won");
                self.show_text("Checkmate! Black Won");
            } else {
                println!("Checkmate! White won");
                self.show_text("Checkmate! White Won");
            }
            self.game_over = true;
        } else if status == EndStates::Stalemate {
            println!("Stalemate");
            self.show_text("Stalemate");
            self.game_over = true;
        }
    }

    fn checkopponent(&mut self, line: &str) {
        let movee = network::decoder3000(line);

        if movee.is_none() {
            println!("REJECTED: {:?}", line);
            self.netty.send("REJECT\n");
            return;
        }

        let (from, to, promo, their_board) = movee.unwrap();

        let board = self.game.board_previewing(from, to, promo);
        let board_ok;
        if board.is_some() {
            board_ok = network::check_same_board(&board.unwrap(), &their_board);
        } else {
            board_ok = false;
        }

        if self.game_over || self.game.turn() == self.my_color {
            self.netty.send("REJECT\n");
            return;
        }

        if !board_ok {
            self.netty.send("REJECT\n");
            return;
        }

        if !self.game.play(from, to, promo) {
            self.netty.send("REJECT\n");
            return;
        }

        self.selected_piece = None;
        self.legalmoves.clear();

        let status = self.game.status();
        if status == EndStates::Checkmate {
            self.netty.send("CHECKMATE\n");
        } else if status == EndStates::Stalemate {
            self.netty.send("STALEMATE\n");
        } else {
            self.netty.send("OK\n");
        }

        self.check_end();

    }
}

impl event::EventHandler<Context, GameError> for ChessGui {
    fn update(&mut self, _ctx: &mut Context) -> GameResult {
        while let Some(line) = self.netty.polla() {
            if line == "DISCONNECTED" {
                println!("opponent left");

                if !(self.game_over) {
                    self.show_text("opponent left");
                    self.game_over = true;
                }
            } else if line == "OK" || line == "CHECKMATE" || line == "STALEMATE" {
                let waiting = self.waiting.take();
                let (from, to, promo) = waiting.unwrap();
                self.actuallyplay(from, to, promo);
                self.check_end();
            } else if line == "REJECT" {
                if self.waiting.take().is_some() {
                    println!("REJECTED FUUUU");
                }
            } else {
                self.checkopponent(&line);
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = graphics::Canvas::from_frame(ctx, PANEL);
        
        // https://doc.rust-lang.org/rust-by-example/flow_control/for.html
        for row in 0..8usize {
            for file in 0..8usize {
                let place = Place { row, file };
                let base: Color;
                if (row + file) % 2 == 0 {
                    base = LIGHT;
                } else {
                    base = DARK;
                };

                canvas.draw(&Quad, DrawParam::new().dest_rect(Rect::new(place.file as f32 * SQUARE_SIZE, place.row as f32 * SQUARE_SIZE, SQUARE_SIZE, SQUARE_SIZE)).color(base));
            }
        }

        // all pieces
        for row in 0..8usize {
            for file in 0..8usize {
                let place = Place{row, file};
                let piece = self.game.piece_at(place);

                if piece.is_some() {
                    let piece = piece.unwrap();
                    let piece = piece.into_ascii();
                    let image = self.sprites.get(&piece);

                    if image.is_some() {
                        let image = image.unwrap();
                        let scale = SQUARE_SIZE / image.width() as f32;

                        canvas.draw(image,DrawParam::new().dest(Vec2::new(file as f32 * SQUARE_SIZE,row as f32 * SQUARE_SIZE,)).scale(Vec2::splat(scale)));
                    }
                }
            }
        }


        // promotion picker
        if self.pending.is_some() {
            canvas.draw(&Quad, DrawParam::new().dest_rect(Rect::new(0.0, 0.0, BOARD, BOARD))
            .color(Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.2,
            }));

            let menu = SQUARE_SIZE * 4.0;
            let x0 = (BOARD - menu) / 2.0;
            let y0 = (BOARD - SQUARE_SIZE) / 2.0;

            let mut i = 0;
            for kind in PROMOTIONS_CHOICESAA.iter() {
                let rect = Rect::new(x0 + i as f32 * SQUARE_SIZE, y0, SQUARE_SIZE, SQUARE_SIZE);
                canvas.draw(&Quad, DrawParam::new().dest_rect(rect).color(LIGHT));

                let bongo;

                if self.game.turn() == ChessColor::White {
                    if *kind == PieceTypes::Queen {
                        bongo = 'Q';
                    } else if *kind == PieceTypes::Rook {
                        bongo = 'R';
                    } else if *kind == PieceTypes::Bishop {
                        bongo = 'B';
                    } else {
                        bongo = 'N';
                    }
                } else {
                    if *kind == PieceTypes::Queen {
                        bongo = 'q';
                    } else if *kind == PieceTypes::Rook {
                        bongo = 'r';
                    } else if *kind == PieceTypes::Bishop {
                        bongo = 'b';
                    } else {
                        bongo = 'n';
                    }
                }

                let bongo = self.sprites.get(&bongo);
                if bongo.is_some() {
                    let bongo = bongo.unwrap();
                    let scale = SQUARE_SIZE / bongo.width() as f32;
                    canvas.draw(bongo,DrawParam::new().dest(Vec2::new(rect.x, rect.y)).scale(Vec2::splat(scale)));
                }
                i += 1;
            }
        }
        

        if let Some(text) = &self.text {
            canvas.draw(text, DrawParam::new().dest(Vec2::new(BOARD / 2.0 - 267.0, BOARD / 2.0)));
        }

        canvas.finish(ctx)?;
        return Ok(())
    }


    fn mouse_button_down_event(&mut self, _ctx: &mut Context, button: MouseButton, x: f32, y: f32) -> GameResult {
        if button != MouseButton::Left {
            return Ok(()); // compilor arg aja baja
        }

        if !(!self.game_over && self.waiting.is_none() && self.game.turn() == self.my_color) {
            return Ok(());
        }

        // om ett move är pending i.e promotion så visa meny
        if self.pending.is_some() {
            let pending = self.pending.unwrap();
            let from = pending.0;
            let to = pending.1;
            let menu = SQUARE_SIZE * 4.0;
            let x0 = (BOARD - menu) / 2.0;
            let y0 = (BOARD - SQUARE_SIZE) / 2.0;

            if x >= x0 && x < x0 + menu && y >= y0 && y < y0 + SQUARE_SIZE {
                let i = ((x - x0) / SQUARE_SIZE) as usize;

                self.send_move(from, to, Some(PROMOTIONS_CHOICESAA[i]));
            }

            return Ok(());
        }

        let clicked = self.place(x, y).unwrap();
        if self.selected_piece.is_some() {
            let from = self.selected_piece.unwrap();
            if self.legalmoves.contains(&clicked) {
                if self.game.is_promotion(from, clicked) {
                    self.pending = Some((from, clicked));
                } else {
                    self.send_move(from, clicked, None);
                    return Ok(());
                }
            }
        }

        self.select(clicked);
        return Ok(())
    }
}
    

fn main() -> GameResult {
    let args: Vec<String> = std::env::args().collect();
    println!("{:?}", args);

    let mode = args.get(1).map(String::as_str);
    let result;

    if mode == Some("host") {
        result = Connection::host();
    } else if mode == Some("join") {
        let addr;
        if args.len() > 2 {
            addr = args[2].as_str();
        } else {
            addr = "127.0.0.1";
        };

        result = Connection::join(addr);

    } else {
        println!("wrong usage idiot");
        panic!();
    };

    let result = result.map_err(|e| {GameError::CustomError(e.to_string())});
    let (netty, my_color) = result?;
    
    let (mut ctx, event_loop) = ggez::ContextBuilder::new("skibidi chess", "alanoo")
        .add_resource_path("./assets")
        .window_setup(ggez::conf::WindowSetup::default().title("Skibidi Chess"))
        .window_mode(ggez::conf::WindowMode::default().dimensions(BOARD, BOARD))
        .build()?;

    let game = ChessGui::new(&mut ctx, netty, my_color)?;
    event::run(ctx, event_loop, game)
}